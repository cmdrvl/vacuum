use std::{fs, path::PathBuf};

mod support;

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

fn canonical_witness_path(home: &std::path::Path) -> PathBuf {
    home.join(".cmdrvl")
        .join("state")
        .join("witness")
        .join("witness.jsonl")
}

#[test]
fn successful_scan_appends_witness_record_by_default() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let witness_path = temp_dir.path().join("witness.jsonl");

    let output = support::vacuum_command("witness-success")
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
fn successful_scan_appends_to_cmdrvl_witness_by_default() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let witness_path = canonical_witness_path(temp_dir.path());

    let output = support::vacuum_command("witness-cmdrvl-default")
        .arg(fixture("simple"))
        .env("HOME", temp_dir.path())
        .env("USERPROFILE", temp_dir.path())
        .env_remove("EPISTEMIC_WITNESS")
        .output()
        .expect("vacuum binary should run");

    assert!(output.status.success(), "scan should exit 0");
    let lines = read_witness_lines(&witness_path);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["tool"], "vacuum");
    assert_eq!(lines[0]["outcome"], "SCAN_COMPLETE");
}

#[test]
fn legacy_home_witness_is_migrated_before_append() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let legacy_path = temp_dir.path().join(".epistemic").join("witness.jsonl");
    fs::create_dir_all(legacy_path.parent().expect("legacy parent"))
        .expect("legacy parent should be created");
    fs::write(
        &legacy_path,
        "{\"version\":\"witness.v0\",\"tool\":\"vacuum\",\"outcome\":\"OLD\"}\n",
    )
    .expect("legacy witness should be written");

    let output = support::vacuum_command("witness-migration")
        .arg(fixture("simple"))
        .env("HOME", temp_dir.path())
        .env("USERPROFILE", temp_dir.path())
        .env_remove("EPISTEMIC_WITNESS")
        .output()
        .expect("vacuum binary should run");

    assert!(output.status.success(), "scan should exit 0");
    let canonical_path = canonical_witness_path(temp_dir.path());
    let lines = read_witness_lines(&canonical_path);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["outcome"], "OLD");
    assert_eq!(lines[1]["outcome"], "SCAN_COMPLETE");

    let notice_path = temp_dir
        .path()
        .join(".cmdrvl")
        .join("notices")
        .join("deprecated-paths.jsonl");
    let notice = fs::read_to_string(notice_path).expect("deprecation notice should be written");
    assert!(notice.contains("legacy_path_migrated"));

    let migration_path = temp_dir
        .path()
        .join(".cmdrvl")
        .join("migrations")
        .join("applied.jsonl");
    let migration = fs::read_to_string(migration_path).expect("migration record should be written");
    assert!(migration.contains("copied_legacy_to_canonical"));
}

#[test]
fn refusal_run_appends_refusal_witness_record() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let witness_path = temp_dir.path().join("witness.jsonl");
    let missing_root = temp_dir.path().join("missing-root");

    let output = support::vacuum_command("witness-refusal")
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
        let output = support::vacuum_command("witness-chain")
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

    let output = support::vacuum_command("witness-no-witness")
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
