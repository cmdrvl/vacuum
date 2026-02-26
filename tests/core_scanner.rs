use std::{path::PathBuf, process::Command};

use serde_json::Value;

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
    let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
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
    let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
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
    let follow = Command::new(env!("CARGO_BIN_EXE_vacuum"))
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

    let no_follow = Command::new(env!("CARGO_BIN_EXE_vacuum"))
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

#[test]
fn mime_mapping_and_path_normalization_are_present_in_records() {
    let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
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

#[test]
fn missing_root_refuses_with_exit_two() {
    let missing_root = PathBuf::from("/definitely-missing-vacuum-root-core-suite");
    let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .arg(missing_root)
        .output()
        .expect("vacuum binary should run");

    assert_eq!(output.status.code(), Some(2), "missing root should refuse");
    let refusal: Value =
        serde_json::from_slice(&output.stdout).expect("refusal stdout should be valid json");
    assert_eq!(refusal["outcome"], "REFUSAL");
    assert_eq!(refusal["refusal"]["code"], "E_ROOT_NOT_FOUND");
}
