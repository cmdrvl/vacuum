use std::process::Command;

#[test]
fn describe_prints_operator_manifest_without_roots() {
    let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .arg("--describe")
        .output()
        .expect("vacuum binary should run");

    assert!(output.status.success(), "describe should exit 0");
    assert!(
        output.stderr.is_empty(),
        "describe should not emit stderr diagnostics"
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let manifest: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("describe should print JSON");

    assert_eq!(manifest["schema_version"], "operator.v0");
    assert_eq!(manifest["name"], "vacuum");
    assert_eq!(manifest["exit_codes"]["0"]["meaning"], "SCAN_COMPLETE");
    assert_eq!(manifest["exit_codes"]["2"]["meaning"], "REFUSAL");
    assert_eq!(manifest["pipeline"]["upstream"], serde_json::json!([]));
    assert_eq!(
        manifest["pipeline"]["downstream"],
        serde_json::json!(["hash"])
    );
    assert!(manifest.get("outcome").is_none());
}
