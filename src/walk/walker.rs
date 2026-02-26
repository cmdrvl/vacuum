use std::{fs, io, path::Path};

use crate::record::builder::VacuumRecord;
use crate::refusal::{codes::RefusalCode, payload::Refusal};
use serde_json::json;

pub fn scan_roots(_roots: &[std::path::PathBuf], _follow_symlinks: bool) -> Vec<VacuumRecord> {
    Vec::new()
}

pub fn validate_roots(roots: &[std::path::PathBuf]) -> Result<(), Refusal> {
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
    use std::fs;

    use super::validate_roots;
    use crate::refusal::codes::RefusalCode;

    #[test]
    fn empty_root_list_returns_not_found_refusal() {
        let refusal = validate_roots(&[]).expect_err("empty roots should fail");

        assert_eq!(refusal.code, RefusalCode::RootNotFound);
        assert_eq!(refusal.detail["root"].as_str(), Some(""));
    }

    #[test]
    fn missing_root_returns_not_found_refusal() {
        let roots = vec![std::path::PathBuf::from("/definitely-missing-vacuum-root")];
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
        let first_missing = std::path::PathBuf::from("/missing-vacuum-root-one");
        let second_missing = std::path::PathBuf::from("/missing-vacuum-root-two");
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
