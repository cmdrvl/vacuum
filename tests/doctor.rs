use std::{path::Path, process::Command};

use serde_json::Value;
use tempfile::TempDir;

fn isolated_command(home: &Path, witness_path: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vacuum"));
    command.env("HOME", home);
    command.env("USERPROFILE", home);
    command.env("EPISTEMIC_WITNESS", witness_path);
    command
}

fn parse_stdout_json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON")
}

#[test]
fn doctor_health_json_exits_zero_without_writing_witness() {
    let home = TempDir::new().expect("temp home should be created");
    let witness_path = home.path().join("witness.jsonl");
    let output = isolated_command(home.path(), &witness_path)
        .args(["doctor", "health", "--json"])
        .output()
        .expect("vacuum doctor should run");

    assert_eq!(output.status.code(), Some(0));
    assert!(
        output.stderr.is_empty(),
        "doctor health should not emit stderr"
    );
    assert!(
        !witness_path.exists(),
        "doctor health must not append or create the witness ledger"
    );

    let report: Value =
        serde_json::from_slice(&output.stdout).expect("doctor health should emit JSON");
    assert_eq!(report["schema_version"], "vacuum.doctor.health.v1");
    assert_eq!(report["tool"], "vacuum");
    assert_eq!(report["read_only"], true);
    assert_eq!(report["ok"], true);
    assert_eq!(report["fixers"], serde_json::json!([]));
}

#[test]
fn doctor_capabilities_json_advertises_no_fixers() {
    let home = TempDir::new().expect("temp home should be created");
    let witness_path = home.path().join("witness.jsonl");
    let output = isolated_command(home.path(), &witness_path)
        .args(["doctor", "capabilities", "--json"])
        .output()
        .expect("vacuum doctor should run");

    assert_eq!(output.status.code(), Some(0));
    let report: Value =
        serde_json::from_slice(&output.stdout).expect("capabilities should emit JSON");

    assert_eq!(report["schema_version"], "vacuum.doctor.capabilities.v1");
    assert_eq!(report["read_only"], true);
    assert_eq!(report["side_effects"]["writes_witness_ledger"], false);
    assert_eq!(report["side_effects"]["scans_roots"], false);
    assert_eq!(report["fix_mode"]["status"], "not_available");
    assert_eq!(report["fixers"], serde_json::json!([]));
}

#[test]
fn doctor_robot_triage_json_is_machine_readable() {
    let home = TempDir::new().expect("temp home should be created");
    let witness_path = home.path().join("witness.jsonl");
    let output = isolated_command(home.path(), &witness_path)
        .args(["doctor", "--robot-triage"])
        .output()
        .expect("vacuum doctor should run");

    assert_eq!(output.status.code(), Some(0));
    let report: Value =
        serde_json::from_slice(&output.stdout).expect("robot triage should emit JSON");

    assert_eq!(report["schema_version"], "vacuum.doctor.triage.v1");
    assert_eq!(report["ok"], true);
    assert_eq!(
        report["health"]["schema_version"],
        "vacuum.doctor.health.v1"
    );
    assert_eq!(
        report["capabilities"]["schema_version"],
        "vacuum.doctor.capabilities.v1"
    );
}

#[test]
fn top_level_robot_triage_json_is_machine_readable() {
    let home = TempDir::new().expect("temp home should be created");
    let witness_path = home.path().join("witness.jsonl");
    let output = isolated_command(home.path(), &witness_path)
        .arg("--robot-triage")
        .output()
        .expect("vacuum robot triage should run");

    assert_eq!(output.status.code(), Some(0));
    assert!(
        output.stderr.is_empty(),
        "top-level robot triage should not emit stderr"
    );
    assert!(
        !witness_path.exists(),
        "top-level robot triage must not append or create the witness ledger"
    );

    let report: Value =
        serde_json::from_slice(&output.stdout).expect("robot triage should emit JSON");
    assert_eq!(report["schema_version"], "vacuum.doctor.triage.v1");
    assert_eq!(report["ok"], true);
    assert_eq!(
        report["capabilities"]["agent_surfaces"]["machine_discovery"],
        "vacuum capabilities --json"
    );
}

#[test]
fn top_level_capabilities_json_is_machine_readable() {
    let home = TempDir::new().expect("temp home should be created");
    let witness_path = home.path().join("witness.jsonl");
    let output = isolated_command(home.path(), &witness_path)
        .args(["capabilities", "--json"])
        .output()
        .expect("vacuum capabilities should run");

    assert_eq!(output.status.code(), Some(0));
    assert!(
        !witness_path.exists(),
        "capabilities must not append or create the witness ledger"
    );

    let report: Value =
        serde_json::from_slice(&output.stdout).expect("capabilities should emit JSON");
    assert_eq!(report["schema_version"], "vacuum.doctor.capabilities.v1");
    assert_eq!(
        report["agent_surfaces"]["robot_triage"],
        "vacuum --robot-triage"
    );
    assert_eq!(report["composition"]["family"]["name"], "cmdrvl-spine");
    assert_eq!(report["composition"]["position"], 1);
    assert_eq!(report["composition"]["produces"][0], "vacuum.v0 JSONL");
    assert!(
        report["composition"]["canonical_chain"][0]
            .as_str()
            .is_some_and(|command| command.contains("vacuum --json <ROOT>... | hashbytes"))
    );
}

#[test]
fn top_level_robot_docs_guide_prints_agent_commands() {
    let home = TempDir::new().expect("temp home should be created");
    let witness_path = home.path().join("witness.jsonl");
    let output = isolated_command(home.path(), &witness_path)
        .args(["robot-docs", "guide"])
        .output()
        .expect("vacuum robot docs should run");

    assert_eq!(output.status.code(), Some(0));
    assert!(
        !witness_path.exists(),
        "robot docs must not append or create the witness ledger"
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("vacuum --robot-triage"));
    assert!(stdout.contains("vacuum capabilities --json"));
    assert!(stdout.contains("vacuum --json <ROOT>..."));
    assert!(stdout.contains("Composition:"));
    assert!(stdout.contains("vacuum --json <ROOT>... | hashbytes | fingerprint --fp <ID>"));
    assert!(stdout.contains("pack seal dataset.lock.json --output evidence/<DATASET>/"));
}

#[test]
fn doctor_fix_is_not_available() {
    let home = TempDir::new().expect("temp home should be created");
    let witness_path = home.path().join("witness.jsonl");
    let output = isolated_command(home.path(), &witness_path)
        .args(["doctor", "--fix"])
        .output()
        .expect("vacuum doctor should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stdout.is_empty(),
        "unknown doctor flags should not emit stdout"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(
        stderr.contains("vacuum doctor --fix is not available"),
        "stderr should explain that --fix is unavailable: {stderr}"
    );
    assert!(
        stderr.contains("vacuum doctor --robot-triage"),
        "stderr should name the safe triage command: {stderr}"
    );
    assert!(
        stderr.contains("vacuum capabilities --json"),
        "stderr should name the safe capabilities command: {stderr}"
    );
    assert!(
        !witness_path.exists(),
        "unavailable fix mode must not create witness state"
    );
}

#[test]
fn describe_runs_without_guard_hooks() {
    let home = TempDir::new().expect("temp home should be created");
    let witness_path = home.path().join("witness.jsonl");
    let output = isolated_command(home.path(), &witness_path)
        .arg("--describe")
        .output()
        .expect("vacuum --describe should run");

    assert_eq!(output.status.code(), Some(0));
    let report = parse_stdout_json(&output);
    assert_eq!(report["name"], "vacuum");
    assert!(
        !witness_path.exists(),
        "--describe must not append or create the witness ledger"
    );
}

#[test]
fn domain_scan_runs_without_guard_hooks() {
    let home = TempDir::new().expect("temp home should be created");
    let scan_root = home.path().join("scan-root");
    std::fs::create_dir_all(&scan_root).expect("scan root should be created");
    std::fs::write(scan_root.join("a.txt"), "hello").expect("scan file should be writable");
    let witness_path = home.path().join("witness.jsonl");

    let output = isolated_command(home.path(), &witness_path)
        .arg(&scan_root)
        .arg("--no-witness")
        .output()
        .expect("vacuum domain command should run");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let first_line = stdout.lines().next().expect("scan should emit a record");
    let record: Value = serde_json::from_str(first_line).expect("record should be JSON");
    assert_eq!(record["version"], "vacuum.v0");
    assert_eq!(record["relative_path"], "a.txt");
    assert!(
        !witness_path.exists(),
        "--no-witness must suppress witness append"
    );
}
