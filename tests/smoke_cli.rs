use std::{path::PathBuf, process::Command};

use serde_json::Value;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn cli_smoke_flags_and_manifest_parse() {
    let version = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .arg("--version")
        .output()
        .expect("version command should run");
    assert_eq!(version.status.code(), Some(0));
    assert!(
        String::from_utf8(version.stdout)
            .expect("version stdout should be utf-8")
            .starts_with("vacuum ")
    );

    let describe = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .arg("--describe")
        .output()
        .expect("describe command should run");
    assert_eq!(describe.status.code(), Some(0));
    let describe_json: Value =
        serde_json::from_slice(&describe.stdout).expect("describe output should parse");
    assert_eq!(describe_json["schema_version"], "operator.v0");

    let schema = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .arg("--schema")
        .output()
        .expect("schema command should run");
    assert_eq!(schema.status.code(), Some(0));
    let schema_json: Value =
        serde_json::from_slice(&schema.stdout).expect("schema output should parse");
    assert_eq!(schema_json["title"], "vacuum.v0");

    let scan = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .arg(fixture("simple"))
        .output()
        .expect("scan command should run");
    assert_eq!(scan.status.code(), Some(0));
    let records = String::from_utf8(scan.stdout)
        .expect("scan stdout should be utf-8")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("manifest line should parse"))
        .collect::<Vec<_>>();
    assert!(!records.is_empty());
}

#[test]
fn cli_smoke_refusal_exit_code() {
    let missing = fixture("does-not-exist");
    let refusal = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .arg(missing)
        .output()
        .expect("refusal command should run");

    assert_eq!(refusal.status.code(), Some(2));
    let envelope: Value =
        serde_json::from_slice(&refusal.stdout).expect("refusal output should parse");
    assert_eq!(envelope["outcome"], "REFUSAL");
}
