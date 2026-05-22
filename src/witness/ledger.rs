use std::{
    env,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use serde_json::{Value, json};

use crate::witness::record::{WitnessRecord, canonical_json};

pub fn append(record: &WitnessRecord) -> std::io::Result<()> {
    ensure_ledger_migrated()?;
    prepare_canonical_tree_from_env(|key| env::var_os(key))?;

    let path = resolve_ledger_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        harden_directory(parent)?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let encoded = canonical_json(record);
    writeln!(file, "{encoded}")?;
    Ok(())
}

pub fn resolve_ledger_path() -> PathBuf {
    resolve_ledger_path_from_env(|key| env::var_os(key))
}

fn resolve_ledger_path_from_env<F>(get_env: F) -> PathBuf
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    if let Some(path) = get_env("EPISTEMIC_WITNESS")
        && !path.is_empty()
    {
        return PathBuf::from(path);
    }

    cmdrvl_root_from_env(get_env)
        .join("state")
        .join("witness")
        .join("witness.jsonl")
}

fn cmdrvl_root_from_env<F>(get_env: F) -> PathBuf
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    if let Some(home) = get_env("HOME")
        .or_else(|| get_env("USERPROFILE"))
        .filter(|value| !value.is_empty())
    {
        return PathBuf::from(home).join(".cmdrvl");
    }

    PathBuf::from(".cmdrvl")
}

pub fn ensure_ledger_migrated() -> std::io::Result<()> {
    ensure_ledger_migrated_from_env(|key| env::var_os(key))
}

fn ensure_ledger_migrated_from_env<F>(get_env: F) -> std::io::Result<()>
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    if get_env("EPISTEMIC_WITNESS").is_some_and(|value| !value.is_empty()) {
        return Ok(());
    }

    let canonical = resolve_ledger_path_from_env(get_env);
    let Some(legacy) = legacy_ledger_paths_from_env(get_env)
        .into_iter()
        .find(|path| path != &canonical && path.exists())
    else {
        return Ok(());
    };

    let root = cmdrvl_root_from_env(get_env);
    prepare_canonical_tree_from_env(get_env)?;
    let notice_path = root.join("notices").join("deprecated-paths.jsonl");
    let migration_path = root.join("migrations").join("applied.jsonl");

    if canonical.exists() {
        append_record_once(
            &notice_path,
            deprecation_record(
                &legacy,
                &canonical,
                "legacy_path_present",
                "canonical_preferred",
            ),
        )?;
        return Ok(());
    }

    if let Some(parent) = canonical.parent() {
        fs::create_dir_all(parent)?;
        harden_directory(parent)?;
    }

    fs::copy(&legacy, &canonical)?;
    let permissions = fs::metadata(&legacy)?.permissions();
    fs::set_permissions(&canonical, permissions)?;

    append_record_once(
        &migration_path,
        migration_record(&legacy, &canonical, "copied_legacy_to_canonical"),
    )?;
    append_record_once(
        &notice_path,
        deprecation_record(
            &legacy,
            &canonical,
            "legacy_path_migrated",
            "canonical_created",
        ),
    )?;

    Ok(())
}

fn prepare_canonical_tree_from_env<F>(get_env: F) -> std::io::Result<()>
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    if get_env("EPISTEMIC_WITNESS").is_some_and(|value| !value.is_empty()) {
        return Ok(());
    }

    let root = cmdrvl_root_from_env(get_env);
    fs::create_dir_all(&root)?;
    harden_directory(&root)
}

fn legacy_ledger_paths_from_env<F>(get_env: F) -> Vec<PathBuf>
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    let mut paths = Vec::new();

    if let Some(home) = get_env("HOME")
        .or_else(|| get_env("USERPROFILE"))
        .filter(|value| !value.is_empty())
    {
        paths.push(PathBuf::from(home).join(".epistemic").join("witness.jsonl"));
    }

    paths.push(PathBuf::from(".epistemic").join("witness.jsonl"));
    paths
}

fn migration_record(source: &Path, destination: &Path, action: &str) -> Value {
    json!({
        "version": "cmdrvl.migration.v1",
        "tool": "vacuum",
        "path_class": "witness_ledger",
        "source_path": source.display().to_string(),
        "destination_path": destination.display().to_string(),
        "action": action,
        "outcome": "ok",
        "secret_values_recorded": false
    })
}

fn deprecation_record(source: &Path, destination: &Path, action: &str, outcome: &str) -> Value {
    json!({
        "version": "cmdrvl.deprecated_path_notice.v1",
        "tool": "vacuum",
        "path_class": "witness_ledger",
        "source_path": source.display().to_string(),
        "destination_path": destination.display().to_string(),
        "action": action,
        "outcome": outcome,
        "secret_values_recorded": false
    })
}

fn append_record_once(path: &PathBuf, record: Value) -> std::io::Result<()> {
    if record_already_exists(path, &record)? {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        harden_directory(parent)?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{record}")?;
    Ok(())
}

fn record_already_exists(path: &PathBuf, record: &Value) -> std::io::Result<bool> {
    let Ok(contents) = fs::read_to_string(path) else {
        return Ok(false);
    };

    Ok(contents.lines().any(|line| {
        let Ok(existing) = serde_json::from_str::<Value>(line) else {
            return false;
        };

        existing.get("tool") == record.get("tool")
            && existing.get("path_class") == record.get("path_class")
            && existing.get("source_path") == record.get("source_path")
            && existing.get("destination_path") == record.get("destination_path")
            && existing.get("action") == record.get("action")
    }))
}

#[cfg(unix)]
fn harden_directory(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn harden_directory(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

pub fn read_prev() -> Option<String> {
    let _ = ensure_ledger_migrated();

    let path = resolve_ledger_path();
    let file = File::open(path).ok()?;
    let reader = std::io::BufReader::new(file);

    let mut last_non_empty = None;
    for line in std::io::BufRead::lines(reader).map_while(Result::ok) {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            last_non_empty = Some(trimmed.to_owned());
        }
    }

    let last = last_non_empty?;
    let value: serde_json::Value = serde_json::from_str(&last).ok()?;
    value.get("id")?.as_str().map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::{ensure_ledger_migrated_from_env, resolve_ledger_path_from_env};
    use std::ffi::OsString;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn empty_epistemic_witness_falls_back_to_cmdrvl_home() {
        let path = resolve_ledger_path_from_env(|key| match key {
            "EPISTEMIC_WITNESS" => Some(OsString::new()),
            "HOME" => Some(OsString::from("/tmp/home")),
            _ => None,
        });

        assert_eq!(
            path,
            PathBuf::from("/tmp/home/.cmdrvl/state/witness/witness.jsonl")
        );
    }

    #[test]
    fn empty_home_falls_back_to_relative_cmdrvl_dir() {
        let path = resolve_ledger_path_from_env(|key| match key {
            "EPISTEMIC_WITNESS" => None,
            "HOME" => Some(OsString::new()),
            "USERPROFILE" => Some(OsString::new()),
            _ => None,
        });

        assert_eq!(path, PathBuf::from(".cmdrvl/state/witness/witness.jsonl"));
    }

    #[test]
    fn explicit_epistemic_witness_override_still_wins() {
        let path = resolve_ledger_path_from_env(|key| match key {
            "EPISTEMIC_WITNESS" => Some(OsString::from("/tmp/custom-witness.jsonl")),
            "HOME" => Some(OsString::from("/tmp/home")),
            _ => None,
        });

        assert_eq!(path, PathBuf::from("/tmp/custom-witness.jsonl"));
    }

    #[test]
    fn migrates_legacy_home_witness_to_cmdrvl_root() {
        let temp = TempDir::new().expect("temp dir should be created");
        let home = temp.path();
        let legacy = home.join(".epistemic").join("witness.jsonl");
        fs::create_dir_all(legacy.parent().expect("legacy parent")).expect("legacy parent");
        fs::write(&legacy, "{\"version\":\"witness.v0\"}\n").expect("legacy ledger");

        ensure_ledger_migrated_from_env(|key| match key {
            "HOME" => Some(home.as_os_str().to_owned()),
            "USERPROFILE" => None,
            "EPISTEMIC_WITNESS" => None,
            _ => None,
        })
        .expect("migration should succeed");

        let canonical = home
            .join(".cmdrvl")
            .join("state")
            .join("witness")
            .join("witness.jsonl");
        assert_eq!(
            fs::read_to_string(&canonical).expect("canonical ledger"),
            "{\"version\":\"witness.v0\"}\n"
        );
        assert!(home.join(".cmdrvl/migrations/applied.jsonl").exists());
        assert!(home.join(".cmdrvl/notices/deprecated-paths.jsonl").exists());
    }

    #[test]
    fn migration_prefers_existing_canonical_ledger_without_overwrite() {
        let temp = TempDir::new().expect("temp dir should be created");
        let home = temp.path();
        let legacy = home.join(".epistemic").join("witness.jsonl");
        let canonical = home
            .join(".cmdrvl")
            .join("state")
            .join("witness")
            .join("witness.jsonl");
        fs::create_dir_all(legacy.parent().expect("legacy parent")).expect("legacy parent");
        fs::create_dir_all(canonical.parent().expect("canonical parent"))
            .expect("canonical parent");
        fs::write(&legacy, "legacy\n").expect("legacy ledger");
        fs::write(&canonical, "canonical\n").expect("canonical ledger");

        ensure_ledger_migrated_from_env(|key| match key {
            "HOME" => Some(home.as_os_str().to_owned()),
            "USERPROFILE" => None,
            "EPISTEMIC_WITNESS" => None,
            _ => None,
        })
        .expect("migration should succeed");

        assert_eq!(
            fs::read_to_string(&canonical).expect("canonical ledger"),
            "canonical\n"
        );
        let notice = fs::read_to_string(home.join(".cmdrvl/notices/deprecated-paths.jsonl"))
            .expect("notice should exist");
        assert!(notice.contains("canonical_preferred"));
    }
}
