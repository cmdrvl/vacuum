use std::{path::PathBuf, process::Command};

use serde_json::Value;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn version_flag_prints_semver_like_output() {
    let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .arg("--version")
        .output()
        .expect("vacuum binary should run");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.trim().starts_with("vacuum "));
}

#[test]
fn describe_and_schema_flags_print_valid_json() {
    let describe = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .arg("--describe")
        .output()
        .expect("vacuum binary should run");
    assert_eq!(describe.status.code(), Some(0));
    let describe_json: Value =
        serde_json::from_slice(&describe.stdout).expect("describe should print json");
    assert_eq!(describe_json["name"], "vacuum");
    assert_eq!(describe_json["schema_version"], "operator.v0");

    let schema = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .arg("--schema")
        .output()
        .expect("vacuum binary should run");
    assert_eq!(schema.status.code(), Some(0));
    let schema_json: Value =
        serde_json::from_slice(&schema.stdout).expect("schema should print json");
    assert_eq!(schema_json["title"], "vacuum.v0");
}

#[test]
fn fixture_scan_stdout_is_jsonl_records() {
    let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .arg(fixture("simple"))
        .output()
        .expect("vacuum binary should run");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(!stdout.trim().is_empty(), "scan should produce records");

    for line in stdout.lines() {
        let record: Value = serde_json::from_str(line).expect("manifest line should parse");
        assert_eq!(record["version"], "vacuum.v0");
        assert!(record["path"].is_string());
        assert!(record["relative_path"].is_string());
    }
}

#[test]
fn missing_root_refusal_returns_exit_two_and_refusal_json() {
    let missing_root = PathBuf::from("/definitely-missing-vacuum-root-smoke");
    let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .arg(missing_root)
        .output()
        .expect("vacuum binary should run");

    assert_eq!(output.status.code(), Some(2));
    let refusal: Value =
        serde_json::from_slice(&output.stdout).expect("refusal stdout should be valid json");
    assert_eq!(refusal["outcome"], "REFUSAL");
    assert_eq!(refusal["refusal"]["code"], "E_ROOT_NOT_FOUND");
}
