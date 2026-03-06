use std::{
    env,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
};

use crate::witness::record::{WitnessRecord, canonical_json};

pub fn append(record: &WitnessRecord) -> std::io::Result<()> {
    let path = resolve_ledger_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
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
    F: Fn(&str) -> Option<OsString>,
{
    if let Some(path) = get_env("EPISTEMIC_WITNESS")
        && !path.is_empty()
    {
        return PathBuf::from(path);
    }

    if let Some(home) = get_env("HOME")
        .or_else(|| get_env("USERPROFILE"))
        .filter(|value| !value.is_empty())
    {
        return PathBuf::from(home).join(".epistemic").join("witness.jsonl");
    }

    PathBuf::from(".epistemic").join("witness.jsonl")
}

pub fn read_prev() -> Option<String> {
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
    use super::resolve_ledger_path_from_env;
    use std::ffi::OsString;
    use std::path::PathBuf;

    #[test]
    fn empty_epistemic_witness_falls_back_to_home() {
        let path = resolve_ledger_path_from_env(|key| match key {
            "EPISTEMIC_WITNESS" => Some(OsString::new()),
            "HOME" => Some(OsString::from("/tmp/home")),
            _ => None,
        });

        assert_eq!(path, PathBuf::from("/tmp/home/.epistemic/witness.jsonl"));
    }

    #[test]
    fn empty_home_falls_back_to_repo_epistemic_dir() {
        let path = resolve_ledger_path_from_env(|key| match key {
            "EPISTEMIC_WITNESS" => None,
            "HOME" => Some(OsString::new()),
            "USERPROFILE" => Some(OsString::new()),
            _ => None,
        });

        assert_eq!(path, PathBuf::from(".epistemic/witness.jsonl"));
    }
}
