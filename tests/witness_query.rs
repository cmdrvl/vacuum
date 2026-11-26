use std::{fs, path::PathBuf, process::Command};

fn write_ledger(path: &PathBuf) {
    let lines = [
        r#"{"version":"witness.v0","tool":"vacuum","outcome":"SCAN_COMPLETE","exit_code":0,"ts":"2026-01-01T00:00:00.000Z","input_hash":"abc123"}"#,
        r#"{"version":"witness.v0","tool":"vacuum","outcome":"REFUSAL","exit_code":2,"ts":"2026-01-02T00:00:00.000Z","input_hash":"def456"}"#,
        r#"{"version":"witness.v0","tool":"other","outcome":"SCAN_COMPLETE","exit_code":0,"ts":"2026-01-03T00:00:00.000Z","input_hash":"ghi789"}"#,
    ];
    fs::write(path, format!("{}\n", lines.join("\n"))).expect("ledger should be written");
}

#[test]
fn witness_query_filters_and_limits_results_in_json_mode() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let ledger_path = temp_dir.path().join("witness.jsonl");
    write_ledger(&ledger_path);

    let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .args([
            "witness",
            "query",
            "--tool",
            "vacuum",
            "--outcome",
            "SCAN_COMPLETE",
            "--limit",
            "1",
            "--json",
        ])
        .env("EPISTEMIC_WITNESS", &ledger_path)
        .output()
        .expect("witness query should run");

    assert!(output.status.success(), "query with matches should exit 0");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let rows: Vec<serde_json::Value> =
        serde_json::from_str(stdout.trim()).expect("query output should be json array");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["tool"], "vacuum");
    assert_eq!(rows[0]["outcome"], "SCAN_COMPLETE");
}

#[test]
fn witness_query_human_mode_renders_rows_and_no_match_exits_zero() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let ledger_path = temp_dir.path().join("witness.jsonl");
    write_ledger(&ledger_path);

    let matched = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .args(["witness", "query", "--tool", "vacuum"])
        .env("EPISTEMIC_WITNESS", &ledger_path)
        .output()
        .expect("witness query should run");
    assert_eq!(matched.status.code(), Some(0));
    let matched_stdout = String::from_utf8(matched.stdout).expect("stdout should be utf-8");
    assert!(matched_stdout.contains("tool=vacuum"));
    assert!(matched_stdout.contains("outcome=SCAN_COMPLETE"));

    let unmatched = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .args(["witness", "query", "--tool", "missing"])
        .env("EPISTEMIC_WITNESS", &ledger_path)
        .output()
        .expect("witness query should run");
    // bd-kxv: an empty result is a successful read (family ADR), not exit 1.
    assert_eq!(
        unmatched.status.code(),
        Some(0),
        "no-match query should exit 0"
    );
}

#[test]
fn witness_last_returns_most_recent_record() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let ledger_path = temp_dir.path().join("witness.jsonl");
    write_ledger(&ledger_path);

    let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .args(["witness", "last", "--json"])
        .env("EPISTEMIC_WITNESS", &ledger_path)
        .output()
        .expect("witness last should run");

    assert!(output.status.success(), "last with entries should exit 0");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let row: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("last output should be json object");
    assert_eq!(row["tool"], "other");
    assert_eq!(row["ts"], "2026-01-03T00:00:00.000Z");
}

#[test]
fn witness_last_human_mode_and_empty_ledger_exit_behavior() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let ledger_path = temp_dir.path().join("witness.jsonl");
    write_ledger(&ledger_path);

    let last = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .args(["witness", "last"])
        .env("EPISTEMIC_WITNESS", &ledger_path)
        .output()
        .expect("witness last should run");
    assert_eq!(last.status.code(), Some(0));
    let last_stdout = String::from_utf8(last.stdout).expect("stdout should be utf-8");
    assert!(last_stdout.contains("tool=other"));
    assert!(last_stdout.contains("outcome=SCAN_COMPLETE"));

    let empty_ledger = temp_dir.path().join("empty-witness.jsonl");
    fs::write(&empty_ledger, "").expect("empty ledger should be created");
    let empty = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .args(["witness", "last"])
        .env("EPISTEMIC_WITNESS", &empty_ledger)
        .output()
        .expect("witness last should run");
    // bd-kxv: an empty ledger is a successful read (family ADR): exit 0, `null` for --json.
    assert_eq!(empty.status.code(), Some(0), "empty ledger should exit 0");
    let empty_json = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .args(["witness", "last", "--json"])
        .env("EPISTEMIC_WITNESS", &empty_ledger)
        .output()
        .expect("witness last should run");
    assert_eq!(empty_json.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&empty_json.stdout).trim(), "null");
}

#[test]
fn witness_count_honors_filters_and_no_match_exits_zero() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let ledger_path = temp_dir.path().join("witness.jsonl");
    write_ledger(&ledger_path);

    let matched = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .args([
            "witness",
            "count",
            "--tool",
            "vacuum",
            "--input-hash",
            "def",
            "--json",
        ])
        .env("EPISTEMIC_WITNESS", &ledger_path)
        .output()
        .expect("witness count should run");
    assert_eq!(matched.status.code(), Some(0));
    let matched_stdout = String::from_utf8(matched.stdout).expect("stdout should be utf-8");
    let matched_json: serde_json::Value =
        serde_json::from_str(matched_stdout.trim()).expect("count output should be json");
    assert_eq!(matched_json["count"], 1);

    let unmatched = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .args(["witness", "count", "--tool", "missing", "--json"])
        .env("EPISTEMIC_WITNESS", &ledger_path)
        .output()
        .expect("witness count should run");
    // bd-kxv: a zero count is a successful read (family ADR): exit 0 and {"count":0}.
    assert_eq!(
        unmatched.status.code(),
        Some(0),
        "no-match count should exit 0"
    );
    assert_eq!(
        String::from_utf8_lossy(&unmatched.stdout).trim(),
        r#"{"count":0}"#
    );

    let human = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .args(["witness", "count", "--tool", "vacuum"])
        .env("EPISTEMIC_WITNESS", &ledger_path)
        .output()
        .expect("witness count should run");
    assert_eq!(human.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(human.stdout)
            .expect("stdout should be utf-8")
            .trim(),
        "2"
    );
}

#[test]
fn witness_human_modes_and_empty_exit_behavior() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let ledger_path = temp_dir.path().join("witness.jsonl");
    write_ledger(&ledger_path);

    let query_human = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .args(["witness", "query", "--tool", "vacuum"])
        .env("EPISTEMIC_WITNESS", &ledger_path)
        .output()
        .expect("witness query should run");
    assert_eq!(query_human.status.code(), Some(0));
    let query_stdout = String::from_utf8(query_human.stdout).expect("stdout should be utf-8");
    assert!(query_stdout.contains("SCAN_COMPLETE"));

    let count_human = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .args(["witness", "count", "--tool", "vacuum"])
        .env("EPISTEMIC_WITNESS", &ledger_path)
        .output()
        .expect("witness count should run");
    assert_eq!(count_human.status.code(), Some(0));
    let count_stdout = String::from_utf8(count_human.stdout).expect("stdout should be utf-8");
    assert_eq!(count_stdout.trim(), "2");

    fs::write(&ledger_path, "").expect("ledger should be emptied");
    let last_empty = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .args(["witness", "last"])
        .env("EPISTEMIC_WITNESS", &ledger_path)
        .output()
        .expect("witness last should run");
    assert_eq!(last_empty.status.code(), Some(0));
}
