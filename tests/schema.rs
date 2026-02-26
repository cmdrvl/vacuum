use std::process::Command;

#[test]
fn schema_prints_valid_contract_without_roots() {
    let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .arg("--schema")
        .output()
        .expect("vacuum binary should run");

    assert!(output.status.success(), "schema should exit 0");
    assert!(
        output.stderr.is_empty(),
        "schema should not emit stderr diagnostics"
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let schema: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("schema should print valid JSON");

    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(schema["title"], "vacuum.v0");
    assert_eq!(schema["properties"]["_skipped"]["type"], "boolean");
    assert_eq!(schema["properties"]["_warnings"]["type"], "array");
    assert!(
        schema["required"]
            .as_array()
            .expect("required should be an array")
            .contains(&serde_json::Value::from("tool_versions"))
    );
}
