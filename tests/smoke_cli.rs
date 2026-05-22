use std::path::PathBuf;

use serde_json::Value;

mod support;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn cli_smoke_flags_and_manifest_parse() {
    let version = support::vacuum_command("smoke-version")
        .arg("--version")
        .output()
        .expect("version command should run");
    assert_eq!(version.status.code(), Some(0));
    assert!(
        String::from_utf8(version.stdout)
            .expect("version stdout should be utf-8")
            .starts_with("vacuum ")
    );

    let describe = support::vacuum_command("smoke-describe")
        .arg("--describe")
        .output()
        .expect("describe command should run");
    assert_eq!(describe.status.code(), Some(0));
    let describe_json: Value =
        serde_json::from_slice(&describe.stdout).expect("describe output should parse");
    assert_eq!(describe_json["schema_version"], "operator.v0");

    let schema = support::vacuum_command("smoke-schema")
        .arg("--schema")
        .output()
        .expect("schema command should run");
    assert_eq!(schema.status.code(), Some(0));
    let schema_json: Value =
        serde_json::from_slice(&schema.stdout).expect("schema output should parse");
    assert_eq!(schema_json["title"], "vacuum.v0");

    let scan = support::vacuum_command("smoke-scan")
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
fn display_flags_short_circuit_before_invalid_witness_args_are_parsed() {
    let version = support::vacuum_command("smoke-short-version")
        .args(["--version", "witness", "query", "--limit", "nope"])
        .output()
        .expect("version command should run");
    assert_eq!(version.status.code(), Some(0));
    assert!(
        String::from_utf8(version.stdout)
            .expect("version stdout should be utf-8")
            .starts_with("vacuum ")
    );
    assert!(version.stderr.is_empty(), "version should not emit stderr");

    let describe = support::vacuum_command("smoke-short-describe")
        .args(["--describe", "witness", "query", "--limit", "nope"])
        .output()
        .expect("describe command should run");
    assert_eq!(describe.status.code(), Some(0));
    let describe_json: Value =
        serde_json::from_slice(&describe.stdout).expect("describe output should parse");
    assert_eq!(describe_json["schema_version"], "operator.v0");
    assert!(
        describe.stderr.is_empty(),
        "describe should not emit stderr"
    );

    let schema = support::vacuum_command("smoke-short-schema")
        .args(["--schema", "witness", "query", "--limit", "nope"])
        .output()
        .expect("schema command should run");
    assert_eq!(schema.status.code(), Some(0));
    let schema_json: Value =
        serde_json::from_slice(&schema.stdout).expect("schema output should parse");
    assert_eq!(schema_json["title"], "vacuum.v0");
    assert!(schema.stderr.is_empty(), "schema should not emit stderr");
}

#[test]
fn cli_smoke_refusal_exit_code() {
    let missing = fixture("does-not-exist");
    let refusal = support::vacuum_command("smoke-refusal")
        .arg(missing)
        .output()
        .expect("refusal command should run");

    assert_eq!(refusal.status.code(), Some(2));
    let envelope: Value =
        serde_json::from_slice(&refusal.stdout).expect("refusal output should parse");
    assert_eq!(envelope["outcome"], "REFUSAL");
}
