use std::{fs, path::PathBuf, process::Command};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn read_witness_lines(path: &PathBuf) -> Vec<serde_json::Value> {
    let contents = fs::read_to_string(path).expect("witness file should be readable");
    contents
        .lines()
        .map(|line| serde_json::from_str(line).expect("witness line should be valid json"))
        .collect()
}

#[test]
fn successful_scan_appends_witness_record_by_default() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let witness_path = temp_dir.path().join("witness.jsonl");

    let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .arg(fixture("simple"))
        .env("EPISTEMIC_WITNESS", &witness_path)
        .output()
        .expect("vacuum binary should run");

    assert!(output.status.success(), "scan should exit 0");
    let lines = read_witness_lines(&witness_path);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["tool"], "vacuum");
    assert_eq!(lines[0]["outcome"], "SCAN_COMPLETE");
    assert_eq!(lines[0]["exit_code"], 0);
}

#[test]
fn refusal_run_appends_refusal_witness_record() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let witness_path = temp_dir.path().join("witness.jsonl");
    let missing_root = temp_dir.path().join("missing-root");

    let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .arg(&missing_root)
        .env("EPISTEMIC_WITNESS", &witness_path)
        .output()
        .expect("vacuum binary should run");

    assert_eq!(output.status.code(), Some(2), "refusal should exit 2");
    let lines = read_witness_lines(&witness_path);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["outcome"], "REFUSAL");
    assert_eq!(lines[0]["exit_code"], 2);
}

#[test]
fn no_witness_flag_suppresses_witness_append() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let witness_path = temp_dir.path().join("witness.jsonl");

    let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .arg(fixture("simple"))
        .arg("--no-witness")
        .env("EPISTEMIC_WITNESS", &witness_path)
        .output()
        .expect("vacuum binary should run");

    assert!(output.status.success(), "scan should exit 0");
    assert!(
        !witness_path.exists(),
        "--no-witness should suppress ledger creation"
    );
}
