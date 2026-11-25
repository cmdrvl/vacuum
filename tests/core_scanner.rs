use std::path::PathBuf;

use serde_json::Value;

mod support;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn parse_json_lines(stdout: &[u8]) -> Vec<Value> {
    String::from_utf8(stdout.to_vec())
        .expect("stdout should be utf-8")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("line should be valid json"))
        .collect()
}

#[test]
fn basic_scan_emits_sorted_manifest_with_exit_zero() {
    let output = support::vacuum_command("core-basic")
        .arg(fixture("nested"))
        .output()
        .expect("vacuum binary should run");

    assert!(output.status.success(), "scan should exit 0");
    let rows = parse_json_lines(&output.stdout);
    let relative_paths = rows
        .iter()
        .map(|row| {
            row["relative_path"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect::<Vec<_>>();

    assert_eq!(
        relative_paths,
        vec!["region/deep/leaf.yaml", "region/north.tsv", "root.txt"]
    );
    assert!(rows.iter().all(|row| row["version"] == "vacuum.v0"));
}

#[test]
fn include_and_exclude_flags_filter_records_by_relative_path() {
    let output = support::vacuum_command("core-filter")
        .arg(fixture("mixed"))
        .args([
            "--include",
            "*.csv",
            "--include",
            "*.md",
            "--exclude",
            "subdir/*",
        ])
        .output()
        .expect("vacuum binary should run");

    assert!(output.status.success(), "filtered scan should exit 0");
    let rows = parse_json_lines(&output.stdout);
    let relative_paths = rows
        .iter()
        .map(|row| row["relative_path"].as_str().unwrap_or_default())
        .collect::<Vec<_>>();

    assert_eq!(relative_paths, vec!["visible.csv"]);
}

#[cfg(unix)]
#[test]
fn symlink_modes_and_skipped_record_contract_hold() {
    let follow = support::vacuum_command("core-follow")
        .arg(fixture("symlinks"))
        .output()
        .expect("vacuum binary should run");
    assert!(follow.status.success());
    let follow_rows = parse_json_lines(&follow.stdout);

    assert!(
        follow_rows
            .iter()
            .any(|row| row["relative_path"] == "dir_link/child.csv")
    );
    let skipped = follow_rows
        .iter()
        .find(|row| row["relative_path"] == "broken_link")
        .expect("broken symlink should be included as skipped");
    assert_eq!(skipped["_skipped"], true);
    assert!(skipped["size"].is_null());
    assert!(skipped["mtime"].is_null());
    assert_eq!(skipped["_warnings"][0]["code"], "E_IO");

    let no_follow = support::vacuum_command("core-no-follow")
        .arg(fixture("symlinks"))
        .arg("--no-follow")
        .output()
        .expect("vacuum binary should run");
    assert!(no_follow.status.success());
    let no_follow_rows = parse_json_lines(&no_follow.stdout);
    assert!(
        !no_follow_rows
            .iter()
            .any(|row| row["relative_path"] == "dir_link/child.csv")
    );
}

#[cfg(unix)]
#[test]
fn multiple_escaping_symlinks_do_not_drop_following_files() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let root = temp_dir.path().join("root");
    let outside = temp_dir.path().join("outside");
    let outside_dir = outside.join("directory");
    std::fs::create_dir_all(&root).expect("root should exist");
    std::fs::create_dir_all(&outside_dir).expect("outside directory should exist");
    let relative_target = outside.join("relative-secret.txt");
    let absolute_target = outside.join("absolute-secret.txt");
    let directory_target = outside_dir.join("directory-secret.txt");
    std::fs::write(&relative_target, "RELATIVE-OUTSIDE-CANARY")
        .expect("relative target should exist");
    std::fs::write(&absolute_target, "ABSOLUTE-OUTSIDE-CANARY")
        .expect("absolute target should exist");
    std::fs::write(&directory_target, "DIRECTORY-OUTSIDE-CANARY")
        .expect("directory target should exist");

    // Create escaping entries before the plain files. The unit test covers the
    // handler directly; this integration case exercises the complete walk.
    symlink(
        "../outside/relative-secret.txt",
        root.join("escape-relative"),
    )
    .expect("relative escaping symlink should exist");
    symlink(&absolute_target, root.join("escape-absolute"))
        .expect("absolute escaping symlink should exist");
    symlink(&outside_dir, root.join("escape-directory"))
        .expect("directory escaping symlink should exist");
    for (name, contents) in [
        ("plain-a.txt", "plain-a"),
        ("plain-b.txt", "plain-b"),
        ("plain-c.txt", "plain-c"),
    ] {
        std::fs::write(root.join(name), contents).expect("plain file should exist");
    }

    let output = support::vacuum_command("core-escape-complete")
        .arg(&root)
        .arg("--no-witness")
        .output()
        .expect("vacuum binary should run");
    assert!(output.status.success(), "scan should exit 0");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(!stdout.contains("RELATIVE-OUTSIDE-CANARY"));
    assert!(!stdout.contains("ABSOLUTE-OUTSIDE-CANARY"));
    assert!(!stdout.contains("DIRECTORY-OUTSIDE-CANARY"));
    let rows = parse_json_lines(stdout.as_bytes());

    let expected_plain = ["plain-a.txt", "plain-b.txt", "plain-c.txt"];
    for relative_path in expected_plain {
        let row = rows
            .iter()
            .find(|row| row["relative_path"] == relative_path)
            .expect("plain file should be present");
        assert_ne!(row["_skipped"], true);
    }

    for relative_path in ["escape-relative", "escape-absolute", "escape-directory"] {
        let row = rows
            .iter()
            .find(|row| row["relative_path"] == relative_path)
            .expect("escaping link should be present");
        assert_eq!(row["_skipped"], true);
        assert_eq!(row["_warnings"][0]["code"], "E_SYMLINK_ESCAPE");
    }

    let relative_paths = rows
        .iter()
        .map(|row| {
            row["relative_path"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect::<Vec<_>>();
    let mut sorted_paths = relative_paths.clone();
    sorted_paths.sort();
    assert_eq!(relative_paths, sorted_paths);
    let outside_prefix = outside.to_string_lossy();
    assert!(rows.iter().all(|row| {
        row["path"]
            .as_str()
            .is_none_or(|path| !path.starts_with(outside_prefix.as_ref()))
    }));
}

#[test]
fn mime_mapping_and_path_normalization_are_present_in_records() {
    let output = support::vacuum_command("core-mime")
        .arg(fixture("nested"))
        .output()
        .expect("vacuum binary should run");
    assert!(output.status.success());

    let rows = parse_json_lines(&output.stdout);
    let row = rows
        .iter()
        .find(|row| row["relative_path"] == "region/deep/leaf.yaml")
        .expect("leaf yaml record should exist");

    assert_eq!(row["extension"], ".yaml");
    assert_eq!(row["mime_guess"], "application/x-yaml");
    assert!(
        row["relative_path"]
            .as_str()
            .is_some_and(|value| !value.contains('\\'))
    );
}

#[cfg(unix)]
#[test]
fn literal_backslashes_in_filenames_are_not_reinterpreted_as_separators() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join(r"literal\slash.csv"), "a,b\n1,2\n").unwrap();

    let output = support::vacuum_command("core-backslashes")
        .arg(tmp.path())
        .output()
        .expect("vacuum binary should run");
    assert!(output.status.success(), "scan should exit 0");

    let rows = parse_json_lines(&output.stdout);
    let row = rows
        .iter()
        .find(|row| row["relative_path"] == r"literal\slash.csv")
        .expect("literal backslash filename should be preserved");

    assert!(
        row["path"]
            .as_str()
            .is_some_and(|value| value.ends_with(r"literal\slash.csv"))
    );
}

#[test]
fn missing_root_refuses_with_exit_two() {
    let missing_root = PathBuf::from("/definitely-missing-vacuum-root-core-suite");
    let output = support::vacuum_command("core-missing-root")
        .arg(missing_root)
        .output()
        .expect("vacuum binary should run");

    assert_eq!(output.status.code(), Some(2), "missing root should refuse");
    let refusal: Value =
        serde_json::from_slice(&output.stdout).expect("refusal stdout should be valid json");
    assert_eq!(refusal["outcome"], "REFUSAL");
    assert_eq!(refusal["refusal"]["code"], "E_ROOT_NOT_FOUND");
}
