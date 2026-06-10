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
fn version_flag_prints_semver_like_output() {
    let output = support::vacuum_command("cli-version")
        .arg("--version")
        .output()
        .expect("vacuum binary should run");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.trim().starts_with("vacuum "));
}

#[test]
fn describe_and_schema_flags_print_valid_json() {
    let describe = support::vacuum_command("cli-describe")
        .arg("--describe")
        .output()
        .expect("vacuum binary should run");
    assert_eq!(describe.status.code(), Some(0));
    let describe_json: Value =
        serde_json::from_slice(&describe.stdout).expect("describe should print json");
    assert_eq!(describe_json["name"], "vacuum");
    assert_eq!(describe_json["schema_version"], "operator.v0");

    let schema = support::vacuum_command("cli-schema")
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
    let output = support::vacuum_command("cli-scan")
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
fn json_flag_is_accepted_as_scan_noop() {
    let output = support::vacuum_command("cli-json-noop")
        .arg("--json")
        .arg(fixture("simple"))
        .arg("--no-witness")
        .output()
        .expect("vacuum binary should run");

    assert_eq!(output.status.code(), Some(0));
    assert!(
        output.stderr.is_empty(),
        "--json scan should not emit diagnostics: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(!stdout.trim().is_empty(), "scan should produce records");

    for line in stdout.lines() {
        let record: Value = serde_json::from_str(line).expect("manifest line should parse");
        assert_eq!(record["version"], "vacuum.v0");
    }
}

#[test]
fn missing_root_refusal_returns_exit_two_and_refusal_json() {
    let missing_root = PathBuf::from("/definitely-missing-vacuum-root-smoke");
    let output = support::vacuum_command("cli-missing-root")
        .arg(missing_root)
        .output()
        .expect("vacuum binary should run");

    assert_eq!(output.status.code(), Some(2));
    let refusal: Value =
        serde_json::from_slice(&output.stdout).expect("refusal stdout should be valid json");
    assert_eq!(refusal["outcome"], "REFUSAL");
    assert_eq!(refusal["refusal"]["code"], "E_ROOT_NOT_FOUND");
    assert_eq!(refusal["refusal"]["next_command"], "ls -la '/'");
}

#[test]
fn empty_roots_refusal_names_default_scan_command() {
    let output = support::vacuum_command("cli-empty-roots")
        .output()
        .expect("vacuum binary should run");

    assert_eq!(output.status.code(), Some(2));
    let refusal: Value =
        serde_json::from_slice(&output.stdout).expect("refusal stdout should be valid json");
    assert_eq!(refusal["outcome"], "REFUSAL");
    assert_eq!(refusal["refusal"]["code"], "E_ROOT_NOT_FOUND");
    assert_eq!(refusal["refusal"]["next_command"], "vacuum .");
}
