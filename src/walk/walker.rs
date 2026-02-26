use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};

use chrono::{SecondsFormat, Utc};
use serde_json::json;
use walkdir::{Error as WalkdirError, WalkDir};

use crate::{
    record::{
        builder::{VacuumRecord, Warning},
        mime::guess_from_extension,
        path::{native_string, normalize_relative},
    },
    refusal::{codes::RefusalCode, payload::Refusal},
};

pub fn scan_roots(roots: &[PathBuf], follow_symlinks: bool) -> Vec<VacuumRecord> {
    scan_roots_with_progress(roots, follow_symlinks, false)
}

pub fn scan_roots_with_progress(
    roots: &[PathBuf],
    follow_symlinks: bool,
    progress_enabled: bool,
) -> Vec<VacuumRecord> {
    let mut records = Vec::new();
    let mut progress = ProgressReporter::new(progress_enabled);

    for root in roots {
        let absolute_root = absolute_root(root);
        let root_value = native_string(&absolute_root);

        for entry in WalkDir::new(&absolute_root)
            .follow_links(follow_symlinks)
            .into_iter()
        {
            match entry {
                Ok(entry) => {
                    if entry.depth() == 0 || entry.file_type().is_dir() {
                        continue;
                    }

                    records.push(build_record(
                        &absolute_root,
                        &root_value,
                        entry.path(),
                        follow_symlinks,
                    ));
                    progress.record_processed();
                    progress.emit_if_due();

                    if let Some(last_record) = records.last() {
                        emit_warning_for_skipped(last_record, progress_enabled);
                    }
                }
                Err(error) => {
                    if let Some(skipped) =
                        build_skipped_from_walk_error(&absolute_root, &root_value, &error)
                    {
                        records.push(skipped);
                        progress.record_processed();
                        progress.emit_if_due();

                        if let Some(last_record) = records.last() {
                            emit_warning_for_skipped(last_record, progress_enabled);
                        }
                    }
                }
            }
        }
    }

    progress.emit_final();
    records
}

pub fn validate_roots(roots: &[PathBuf]) -> Result<(), Refusal> {
    if roots.is_empty() {
        return Err(Refusal::new(
            RefusalCode::RootNotFound,
            json!({ "root": "" }),
        ));
    }

    for root in roots {
        validate_root(root)?;
    }
    Ok(())
}

fn build_record(
    root: &Path,
    root_value: &str,
    entry_path: &Path,
    follow_symlinks: bool,
) -> VacuumRecord {
    let relative_path = match entry_path.strip_prefix(root) {
        Ok(relative) => normalize_relative(relative),
        Err(_) => normalize_relative(entry_path),
    };

    let extension = entry_path
        .extension()
        .map(|value| format!(".{}", value.to_string_lossy()));
    let mime_guess = guess_from_extension(extension.as_deref()).map(str::to_string);

    let path_value = native_string(entry_path);

    let mut record = VacuumRecord::empty();
    record.path = path_value;
    record.relative_path = relative_path;
    record.root = root_value.to_string();
    record.extension = extension;
    record.mime_guess = mime_guess;

    let metadata = if follow_symlinks {
        fs::metadata(entry_path)
    } else {
        fs::symlink_metadata(entry_path)
    };

    match metadata {
        Ok(metadata) => {
            let output_path = if follow_symlinks {
                fs::canonicalize(entry_path).unwrap_or_else(|_| entry_path.to_path_buf())
            } else {
                entry_path.to_path_buf()
            };
            record.path = native_string(&output_path);
            record.size = Some(metadata.len());
            record.mtime = format_mtime(metadata.modified().ok());
        }
        Err(error) => {
            record._skipped = Some(true);
            record._warnings = Some(vec![io_warning(
                format!("Cannot read file metadata: {error}"),
                error.to_string(),
            )]);
        }
    }

    record
}

fn format_mtime(value: Option<SystemTime>) -> Option<String> {
    value.map(|mtime| {
        chrono::DateTime::<Utc>::from(mtime).to_rfc3339_opts(SecondsFormat::Millis, true)
    })
}

fn build_skipped_from_walk_error(
    root: &Path,
    root_value: &str,
    error: &WalkdirError,
) -> Option<VacuumRecord> {
    let path = error.path()?;
    if path == root {
        return None;
    }

    let relative_path = match path.strip_prefix(root) {
        Ok(relative) => normalize_relative(relative),
        Err(_) => normalize_relative(path),
    };

    let extension = path
        .extension()
        .map(|value| format!(".{}", value.to_string_lossy()));
    let mime_guess = guess_from_extension(extension.as_deref()).map(str::to_string);

    let mut record = VacuumRecord::empty();
    record.path = native_string(path);
    record.relative_path = relative_path;
    record.root = root_value.to_string();
    record.extension = extension;
    record.mime_guess = mime_guess;
    record._skipped = Some(true);
    record._warnings = Some(vec![io_warning(
        format!("Cannot read directory entry: {error}"),
        error.to_string(),
    )]);

    Some(record)
}

fn io_warning(message: String, error: String) -> Warning {
    Warning {
        tool: "vacuum".to_string(),
        code: "E_IO".to_string(),
        message,
        detail: json!({ "error": error }),
    }
}

fn emit_warning_for_skipped(record: &VacuumRecord, progress_enabled: bool) {
    if record._skipped != Some(true) {
        return;
    }

    let warning = match record
        ._warnings
        .as_ref()
        .and_then(|warnings| warnings.first())
    {
        Some(warning) => warning,
        None => return,
    };

    if progress_enabled {
        let payload = json!({
            "type": "warning",
            "tool": "vacuum",
            "path": record.path,
            "message": warning.message,
        });
        eprintln!("{payload}");
    } else {
        eprintln!("vacuum: skipped {} ({})", record.path, warning.message);
    }
}

struct ProgressReporter {
    enabled: bool,
    processed: u64,
    started_at: Instant,
    last_emitted_at: Instant,
}

impl ProgressReporter {
    fn new(enabled: bool) -> Self {
        let now = Instant::now();
        Self {
            enabled,
            processed: 0,
            started_at: now,
            last_emitted_at: now,
        }
    }

    fn record_processed(&mut self) {
        self.processed = self.processed.saturating_add(1);
    }

    fn emit_if_due(&mut self) {
        if !self.enabled {
            return;
        }

        let elapsed = self.last_emitted_at.elapsed();
        if self.processed.is_multiple_of(1000) || elapsed >= Duration::from_millis(500) {
            self.emit();
        }
    }

    fn emit_final(&mut self) {
        if !self.enabled {
            return;
        }

        self.emit();
    }

    fn emit(&mut self) {
        self.last_emitted_at = Instant::now();
        let payload = json!({
            "type": "progress",
            "tool": "vacuum",
            "processed": self.processed,
            "total": serde_json::Value::Null,
            "elapsed_ms": self.started_at.elapsed().as_millis() as u64,
        });
        eprintln!("{payload}");
    }
}

fn absolute_root(root: &Path) -> PathBuf {
    if root.is_absolute() {
        return root.to_path_buf();
    }

    match std::env::current_dir() {
        Ok(current_dir) => current_dir.join(root),
        Err(_) => root.to_path_buf(),
    }
}

fn validate_root(root: &Path) -> Result<(), Refusal> {
    let metadata = match fs::metadata(root) {
        Ok(metadata) => metadata,
        Err(error) => return Err(refusal_from_io(root, error)),
    };

    if !metadata.is_dir() {
        return Err(Refusal::new(
            RefusalCode::Io,
            json!({
                "root": root.display().to_string(),
                "error": "Not a directory",
            }),
        ));
    }

    if let Err(error) = fs::read_dir(root) {
        return Err(refusal_from_io(root, error));
    }

    Ok(())
}

fn refusal_from_io(root: &Path, error: io::Error) -> Refusal {
    match error.kind() {
        io::ErrorKind::NotFound => Refusal::new(
            RefusalCode::RootNotFound,
            json!({
                "root": root.display().to_string(),
            }),
        ),
        io::ErrorKind::PermissionDenied => Refusal::new(
            RefusalCode::RootPermission,
            json!({
                "root": root.display().to_string(),
                "error": error.to_string(),
            }),
        ),
        _ => Refusal::new(
            RefusalCode::Io,
            json!({
                "root": root.display().to_string(),
                "error": error.to_string(),
            }),
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use super::{scan_roots, validate_roots};
    use crate::refusal::codes::RefusalCode;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }

    #[test]
    fn scan_collects_nested_files_recursively() {
        let root = fixture("nested");
        let records = scan_roots(&[root], true);

        let mut relative_paths = records
            .iter()
            .map(|record| record.relative_path.clone())
            .collect::<Vec<_>>();
        relative_paths.sort();

        assert_eq!(
            relative_paths,
            vec!["region/deep/leaf.yaml", "region/north.tsv", "root.txt"]
        );
    }

    #[test]
    fn scan_supports_multiple_roots() {
        let roots = vec![fixture("simple"), fixture("nested")];
        let records = scan_roots(&roots, true);

        assert!(records.len() >= 6);
        assert!(
            records
                .iter()
                .any(|record| record.relative_path == "alpha.csv")
        );
        assert!(
            records
                .iter()
                .any(|record| record.relative_path == "root.txt")
        );
    }

    #[cfg(unix)]
    #[test]
    fn follows_directory_symlinks_by_default() {
        let root = fixture("symlinks");
        let records = scan_roots(&[root], true);

        assert!(
            records
                .iter()
                .any(|record| record.relative_path == "dir_link/child.csv")
        );

        let linked_file = records
            .iter()
            .find(|record| record.relative_path == "file_link.txt")
            .expect("file symlink should be included");
        assert!(linked_file.path.ends_with("target.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn no_follow_does_not_traverse_directory_symlinks() {
        let root = fixture("symlinks");
        let records = scan_roots(&[root], false);

        assert!(
            !records
                .iter()
                .any(|record| record.relative_path == "dir_link/child.csv")
        );
        assert!(
            records
                .iter()
                .any(|record| record.relative_path == "dir_target/child.csv")
        );

        let linked_file = records
            .iter()
            .find(|record| record.relative_path == "file_link.txt")
            .expect("file symlink should be included");
        assert!(linked_file.path.ends_with("file_link.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn broken_file_symlink_is_emitted_as_skipped_record() {
        let root = fixture("symlinks");
        let records = scan_roots(std::slice::from_ref(&root), true);

        let skipped = records
            .iter()
            .find(|record| record.relative_path == "broken_link")
            .expect("broken symlink should be represented");

        assert_eq!(skipped._skipped, Some(true));
        assert_eq!(skipped.size, None);
        assert_eq!(skipped.mtime, None);
        assert_eq!(skipped.root, root.to_string_lossy());

        let warning = skipped
            ._warnings
            .as_ref()
            .and_then(|warnings| warnings.first())
            .expect("skipped record should include warning");
        assert_eq!(warning.tool, "vacuum");
        assert_eq!(warning.code, "E_IO");
        assert!(warning.message.contains("Cannot read"));
        assert!(warning.detail["error"].is_string());
    }

    #[cfg(unix)]
    #[test]
    fn skipped_record_preserves_extension_and_mime_when_derivable() {
        use std::os::unix::fs::symlink;

        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let root = temp_dir.path().join("root");
        fs::create_dir_all(&root).expect("root should be created");
        symlink(Path::new("missing.csv"), root.join("broken.csv"))
            .expect("broken symlink should be created");

        let records = scan_roots(std::slice::from_ref(&root), true);
        let skipped = records
            .iter()
            .find(|record| record.relative_path == "broken.csv")
            .expect("broken csv symlink should be represented");

        assert_eq!(skipped._skipped, Some(true));
        assert_eq!(skipped.extension.as_deref(), Some(".csv"));
        assert_eq!(skipped.mime_guess.as_deref(), Some("text/csv"));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_cycles_do_not_crash_scan() {
        use std::os::unix::fs::symlink;

        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        let root = temp_dir.path().join("root");
        let loop_dir = root.join("loop");
        fs::create_dir_all(&loop_dir).expect("loop directory should be created");
        fs::write(loop_dir.join("payload.txt"), "cycle-safe").expect("payload file should exist");
        symlink(&root, loop_dir.join("cycle")).expect("cycle symlink should be created");

        let records = scan_roots(&[root], true);

        assert!(
            records
                .iter()
                .any(|record| record.relative_path == "loop/payload.txt")
        );
    }

    #[test]
    fn empty_root_list_returns_not_found_refusal() {
        let refusal = validate_roots(&[]).expect_err("empty roots should fail");

        assert_eq!(refusal.code, RefusalCode::RootNotFound);
        assert_eq!(refusal.detail["root"].as_str(), Some(""));
    }

    #[test]
    fn missing_root_returns_not_found_refusal() {
        let roots = vec![PathBuf::from("/definitely-missing-vacuum-root")];
        let refusal = validate_roots(&roots).expect_err("missing root should fail");

        assert_eq!(refusal.code, RefusalCode::RootNotFound);
        assert_eq!(
            refusal.detail["root"].as_str(),
            Some("/definitely-missing-vacuum-root")
        );
        assert!(refusal.detail["error"].is_null());
    }

    #[test]
    fn non_directory_root_returns_io_refusal() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let file_path = temp_dir.path().join("single-file-root");
        fs::write(&file_path, "x").expect("fixture file should be created");

        let roots = vec![file_path.clone()];
        let refusal = validate_roots(&roots).expect_err("file root should fail");

        assert_eq!(refusal.code, RefusalCode::Io);
        assert_eq!(refusal.detail["root"].as_str(), file_path.to_str());
        assert_eq!(refusal.detail["error"].as_str(), Some("Not a directory"));
    }

    #[test]
    fn validation_is_fail_fast_in_root_order() {
        let first_missing = PathBuf::from("/missing-vacuum-root-one");
        let second_missing = PathBuf::from("/missing-vacuum-root-two");
        let roots = vec![first_missing.clone(), second_missing];

        let refusal = validate_roots(&roots).expect_err("first invalid root should fail");

        assert_eq!(refusal.code, RefusalCode::RootNotFound);
        assert_eq!(refusal.detail["root"].as_str(), first_missing.to_str());
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_root_returns_permission_refusal() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let root = temp_dir.path().join("restricted");
        fs::create_dir(&root).expect("restricted root should be created");

        let original_permissions = fs::metadata(&root)
            .expect("metadata should be readable")
            .permissions();
        let mut restricted_permissions = original_permissions.clone();
        restricted_permissions.set_mode(0o000);
        fs::set_permissions(&root, restricted_permissions)
            .expect("permissions should be restricted");

        let roots = vec![root.clone()];
        let refusal = validate_roots(&roots).expect_err("restricted root should fail");

        let mut restore_permissions = original_permissions;
        restore_permissions.set_mode(0o755);
        fs::set_permissions(&root, restore_permissions).expect("permissions should be restored");

        assert_eq!(refusal.code, RefusalCode::RootPermission);
        assert_eq!(refusal.detail["root"].as_str(), root.to_str());
    }
}
