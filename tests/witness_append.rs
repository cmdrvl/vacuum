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
    let record = &lines[0];
    assert!(
        record["id"]
            .as_str()
            .is_some_and(|value| value.starts_with("blake3:"))
    );
    assert_eq!(record["tool"], "vacuum");
    assert_eq!(record["version"], env!("CARGO_PKG_VERSION"));
    assert!(
        record["binary_hash"]
            .as_str()
            .is_some_and(|value| value.starts_with("blake3:"))
    );
    assert_eq!(record["outcome"], "SCAN_COMPLETE");
    assert_eq!(record["exit_code"], 0);
    assert_eq!(
        record["inputs"][0]["path"],
        fixture("simple").to_string_lossy().as_ref()
    );
    assert!(record["inputs"][0]["hash"].is_null());
    assert!(record["inputs"][0]["bytes"].is_null());
    assert!(record["prev"].is_null());
    assert!(
        record["output_hash"]
            .as_str()
            .is_some_and(|value| value.starts_with("blake3:"))
    );
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
    assert_eq!(
        lines[0]["inputs"][0]["path"],
        missing_root.to_string_lossy().as_ref()
    );
    assert!(
        lines[0]["output_hash"]
            .as_str()
            .is_some_and(|value| value.starts_with("blake3:"))
    );
}

#[test]
fn consecutive_runs_chain_prev_ids() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let witness_path = temp_dir.path().join("witness.jsonl");
    let root = fixture("simple");

    for _ in 0..2 {
        let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
            .arg(&root)
            .env("EPISTEMIC_WITNESS", &witness_path)
            .output()
            .expect("vacuum binary should run");
        assert!(output.status.success(), "scan should exit 0");
    }

    let lines = read_witness_lines(&witness_path);
    assert_eq!(lines.len(), 2);
    assert!(lines[0]["prev"].is_null());
    assert_eq!(lines[1]["prev"], lines[0]["id"]);
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
