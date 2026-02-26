use std::{
    fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use chrono::{SecondsFormat, Utc};
use serde_json::json;
use walkdir::WalkDir;

use crate::{
    record::{
        builder::VacuumRecord,
        mime::guess_from_extension,
        path::{native_string, normalize_relative},
    },
    refusal::{codes::RefusalCode, payload::Refusal},
};

pub fn scan_roots(roots: &[PathBuf], follow_symlinks: bool) -> Vec<VacuumRecord> {
    let mut records = Vec::new();

    for root in roots {
        let absolute_root = absolute_root(root);
        let root_value = native_string(&absolute_root);

        for entry in WalkDir::new(&absolute_root)
            .follow_links(follow_symlinks)
            .into_iter()
            .filter_map(Result::ok)
        {
            if entry.depth() == 0 || entry.file_type().is_dir() {
                continue;
            }

            if let Some(record) =
                build_record(&absolute_root, &root_value, entry.path(), follow_symlinks)
            {
                records.push(record);
            }
        }
    }

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
) -> Option<VacuumRecord> {
    let metadata = if follow_symlinks {
        fs::metadata(entry_path).ok()?
    } else {
        fs::symlink_metadata(entry_path).ok()?
    };

    let relative_path = match entry_path.strip_prefix(root) {
        Ok(relative) => normalize_relative(relative),
        Err(_) => normalize_relative(entry_path),
    };

    let extension = entry_path
        .extension()
        .map(|value| format!(".{}", value.to_string_lossy()));
    let mime_guess = guess_from_extension(extension.as_deref()).map(str::to_string);

    let output_path = if follow_symlinks {
        fs::canonicalize(entry_path).unwrap_or_else(|_| entry_path.to_path_buf())
    } else {
        entry_path.to_path_buf()
    };

    let mut record = VacuumRecord::empty();
    record.path = native_string(&output_path);
    record.relative_path = relative_path;
    record.root = root_value.to_string();
    record.size = Some(metadata.len());
    record.mtime = format_mtime(metadata.modified().ok());
    record.extension = extension;
    record.mime_guess = mime_guess;

    Some(record)
}

fn format_mtime(value: Option<SystemTime>) -> Option<String> {
    value.map(|mtime| {
        chrono::DateTime::<Utc>::from(mtime).to_rfc3339_opts(SecondsFormat::Millis, true)
    })
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
    use std::{fs, path::PathBuf};

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
