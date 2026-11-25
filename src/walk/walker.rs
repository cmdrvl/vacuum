use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};

use chrono::{SecondsFormat, Utc};
use serde_json::json;
use walkdir::{DirEntry, Error as WalkdirError, WalkDir};

use crate::{
    record::{
        builder::{VacuumRecord, Warning},
        mime::guess_from_extension,
        path::{native_string, normalize_relative},
    },
    refusal::{codes::RefusalCode, payload::Refusal},
};

const E_SYMLINK_ESCAPE: &str = "E\x5fSYMLINK_ESCAPE";

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
        let containment_root =
            fs::canonicalize(&absolute_root).unwrap_or_else(|_| absolute_root.clone());
        let root_value = native_string(&absolute_root);

        let mut walker = WalkDir::new(&absolute_root)
            .follow_links(follow_symlinks)
            .into_iter();
        while let Some(entry) = walker.next() {
            match entry {
                Ok(entry) => {
                    let action = handle_entry(
                        &absolute_root,
                        &root_value,
                        &containment_root,
                        &entry,
                        follow_symlinks,
                    );
                    if action.skip_current_dir {
                        walker.skip_current_dir();
                    }
                    if let Some(record) = action.record {
                        append_record(&mut records, &mut progress, record, progress_enabled);
                    }
                }
                Err(error) => {
                    if let Some(skipped) =
                        build_skipped_from_walk_error(&absolute_root, &root_value, &error)
                    {
                        append_record(&mut records, &mut progress, skipped, progress_enabled);
                    }
                }
            }
        }
    }

    progress.emit_final();
    records
}

#[derive(Debug, PartialEq, Eq)]
struct EntryAction {
    record: Option<VacuumRecord>,
    skip_current_dir: bool,
}

fn handle_entry(
    root: &Path,
    root_value: &str,
    containment_root: &Path,
    entry: &DirEntry,
    follow_symlinks: bool,
) -> EntryAction {
    if follow_symlinks
        && entry.path_is_symlink()
        && let Ok(target) = fs::canonicalize(entry.path())
        && !target.starts_with(containment_root)
    {
        let skip_current_dir = fs::metadata(entry.path())
            .map(|metadata| metadata.is_dir())
            .unwrap_or(false);
        return EntryAction {
            record: Some(build_symlink_escape_record(
                root,
                root_value,
                entry.path(),
                &target,
            )),
            skip_current_dir,
        };
    }

    if entry.depth() == 0 || entry.file_type().is_dir() {
        return EntryAction {
            record: None,
            skip_current_dir: false,
        };
    }

    EntryAction {
        record: Some(build_record(
            root,
            root_value,
            entry.path(),
            follow_symlinks,
        )),
        skip_current_dir: false,
    }
}

fn append_record(
    records: &mut Vec<VacuumRecord>,
    progress: &mut ProgressReporter,
    record: VacuumRecord,
    progress_enabled: bool,
) {
    records.push(record);
    progress.record_processed();
    progress.emit_if_due();

    if let Some(last_record) = records.last() {
        emit_warning_for_skipped(last_record, progress_enabled);
    }
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

fn build_symlink_escape_record(
    root: &Path,
    root_value: &str,
    entry_path: &Path,
    target: &Path,
) -> VacuumRecord {
    let relative_path = match entry_path.strip_prefix(root) {
        Ok(relative) => normalize_relative(relative),
        Err(_) => normalize_relative(entry_path),
    };
    let extension = entry_path
        .extension()
        .map(|value| format!(".{}", value.to_string_lossy()));
    let mime_guess = guess_from_extension(extension.as_deref()).map(str::to_string);

    let mut record = VacuumRecord::empty();
    record.path = native_string(entry_path);
    record.relative_path = relative_path;
    record.root = root_value.to_string();
    record.extension = extension;
    record.mime_guess = mime_guess;
    record._skipped = Some(true);
    record._warnings = Some(vec![Warning {
        tool: "vacuum".to_string(),
        code: E_SYMLINK_ESCAPE.to_string(),
        message: format!("Skipped symlink outside scan root: {}", target.display()),
        detail: json!({
            "root": root.display().to_string(),
            "target": target.display().to_string(),
        }),
    }]);
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
        let suggested_root = suggested_scan_root(root);
        return Err(Refusal::new(
            RefusalCode::Io,
            json!({
                "root": root.display().to_string(),
                "error": "Not a directory",
                "suggested_root": display_root(suggested_root),
            }),
        )
        .with_next_command(format!("vacuum {}", shell_quote_root(suggested_root))));
    }

    if let Err(error) = fs::read_dir(root) {
        return Err(refusal_from_io(root, error));
    }

    Ok(())
}

fn suggested_scan_root(root: &Path) -> &Path {
    match root.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

fn display_root(root: &Path) -> String {
    if root.as_os_str().is_empty() {
        ".".to_string()
    } else {
        root.display().to_string()
    }
}

fn shell_quote_root(root: &Path) -> String {
    format!("'{}'", display_root(root).replace('\'', "'\"'\"'"))
}

fn refusal_from_io(root: &Path, error: io::Error) -> Refusal {
    match error.kind() {
        io::ErrorKind::NotFound => Refusal::new(
            RefusalCode::RootNotFound,
            json!({
                "root": root.display().to_string(),
            }),
        )
        .with_next_command(format!(
            "ls -la {}",
            shell_quote_root(suggested_scan_root(root))
        )),
        io::ErrorKind::PermissionDenied => Refusal::new(
            RefusalCode::RootPermission,
            json!({
                "root": root.display().to_string(),
                "error": error.to_string(),
            }),
        )
        .with_next_command(format!("ls -ld {}", shell_quote_root(root))),
        _ => Refusal::new(
            RefusalCode::Io,
            json!({
                "root": root.display().to_string(),
                "error": error.to_string(),
            }),
        )
        .with_next_command(format!("ls -ld {}", shell_quote_root(root))),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use walkdir::WalkDir;

    use super::{
        E_SYMLINK_ESCAPE, EntryAction, VacuumRecord, build_skipped_from_walk_error, handle_entry,
        scan_roots, suggested_scan_root, validate_roots,
    };
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
    fn outside_root_symlink_is_skipped_with_explicit_escape_warning() {
        use std::os::unix::fs::symlink;

        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let root = temp_dir.path().join("root");
        let outside = temp_dir.path().join("outside");
        fs::create_dir_all(&root).expect("root should be created");
        fs::create_dir_all(&outside).expect("outside directory should be created");
        let protected = outside.join("protected.txt");
        fs::write(&protected, "protected").expect("protected file should be created");
        let canonical_protected =
            fs::canonicalize(&protected).expect("protected path should resolve");
        let link = root.join("escape.txt");
        symlink(&protected, &link).expect("escaping symlink should be created");

        let records = scan_roots(std::slice::from_ref(&root), true);
        let escaped = records
            .iter()
            .find(|record| record.relative_path == "escape.txt")
            .expect("escaping symlink should be represented");

        assert_eq!(escaped._skipped, Some(true));
        assert_eq!(escaped.size, None);
        assert_eq!(escaped.mtime, None);
        let warning = escaped
            ._warnings
            .as_ref()
            .and_then(|warnings| warnings.first())
            .expect("escaping symlink should include a warning");
        assert_eq!(warning.code, "E_SYMLINK_ESCAPE");
        assert_eq!(warning.detail["root"], root.display().to_string());
        assert_eq!(
            warning.detail["target"],
            canonical_protected.display().to_string()
        );
        assert!(!records.iter().any(|record| {
            record.path == protected.display().to_string() && record._skipped != Some(true)
        }));
    }

    #[cfg(unix)]
    #[test]
    fn entry_handler_emits_file_escape_without_skipping_following_entries() {
        use std::os::unix::fs::symlink;

        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let root = temp_dir.path().join("root");
        let outside = temp_dir.path().join("outside");
        fs::create_dir_all(&root).expect("root should be created");
        fs::create_dir_all(&outside).expect("outside directory should be created");
        let protected = outside.join("protected.txt");
        fs::write(&protected, "outside-canary").expect("protected file should be created");
        let link = root.join("escape-first");
        symlink(&protected, &link).expect("escaping symlink should be created");
        let plain = root.join("plain-after.txt");
        fs::write(&plain, "plain").expect("plain file should be created");

        let containment_root = fs::canonicalize(&root).expect("root should resolve");
        let root_value = root.to_string_lossy().to_string();
        let escape_entry = WalkDir::new(&root)
            .follow_links(true)
            .into_iter()
            .filter_map(Result::ok)
            .find(|entry| entry.path() == link)
            .expect("escaping entry should be found");

        let action = handle_entry(&root, &root_value, &containment_root, &escape_entry, true);
        assert!(
            action.record.is_some(),
            "escaping file should produce a record"
        );
        let skip_current_dir = action.skip_current_dir;
        let record = action
            .record
            .expect("escaping file should produce a record");
        assert!(!skip_current_dir);
        assert_eq!(record._skipped, Some(true));
        assert_eq!(record._warnings.as_ref().unwrap()[0].code, E_SYMLINK_ESCAPE);

        let plain_entry = WalkDir::new(&root)
            .follow_links(true)
            .into_iter()
            .filter_map(Result::ok)
            .find(|entry| entry.path() == plain)
            .expect("plain entry should be found after escape");
        let action = handle_entry(&root, &root_value, &containment_root, &plain_entry, true);
        assert!(
            action.record.is_some(),
            "plain file should produce a record"
        );
        let skip_current_dir = action.skip_current_dir;
        let record = action.record.expect("plain file should produce a record");
        assert!(!skip_current_dir);
        assert_ne!(record._skipped, Some(true));
    }

    #[cfg(unix)]
    #[test]
    fn entry_handler_emits_cannot_read_skip_and_continues() {
        use std::os::unix::fs::symlink;

        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let root = temp_dir.path().join("root");
        fs::create_dir_all(&root).expect("root should be created");
        let broken = root.join("broken-first");
        symlink("missing.txt", &broken).expect("broken symlink should be created");
        let plain = root.join("plain-after.txt");
        fs::write(&plain, "plain").expect("plain file should be created");

        let containment_root = fs::canonicalize(&root).expect("root should resolve");
        let root_value = root.to_string_lossy().to_string();
        let broken_error = WalkDir::new(&root)
            .follow_links(true)
            .into_iter()
            .filter_map(Result::err)
            .find(|error| error.path() == Some(broken.as_path()))
            .expect("broken entry should produce a walk error");
        let record = build_skipped_from_walk_error(&root, &root_value, &broken_error)
            .expect("broken file should produce a record");
        assert_eq!(record._skipped, Some(true));
        assert_eq!(record._warnings.as_ref().unwrap()[0].code, "E_IO");

        let plain_entry = WalkDir::new(&root)
            .follow_links(true)
            .into_iter()
            .filter_map(Result::ok)
            .find(|entry| entry.path() == plain)
            .expect("plain entry should be found after broken link");
        assert!(matches!(
            handle_entry(&root, &root_value, &containment_root, &plain_entry, true,),
            EntryAction {
                record: Some(VacuumRecord { _skipped: None, .. }),
                skip_current_dir: false,
            }
        ));
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
        assert_eq!(
            refusal.detail["suggested_root"].as_str(),
            temp_dir.path().to_str()
        );
        let expected = format!("vacuum '{}'", temp_dir.path().display());
        assert_eq!(refusal.next_command.as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn suggested_scan_root_uses_parent_directory_when_available() {
        let file_path = PathBuf::from("/tmp/vacuum/report.json");

        assert_eq!(suggested_scan_root(&file_path), Path::new("/tmp/vacuum"));
    }

    #[test]
    fn suggested_scan_root_falls_back_to_dot_for_single_component_relative_file() {
        let file_path = PathBuf::from("analysis_results.json");

        assert_eq!(suggested_scan_root(&file_path), Path::new("."));
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
