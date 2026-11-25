use std::{path::Path, process::Command};

use serde_json::Value;
use spine_rules::{contract::OPERATOR_V1_SCHEMA, doctor::DOCTOR_V1_SCHEMA};
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

fn schema_errors(instance: &Value, schema_text: &str) -> Vec<String> {
    let schema: Value = serde_json::from_str(schema_text).expect("spine schema should be JSON");
    let validator = jsonschema::validator_for(&schema).expect("spine schema should compile");
    validator
        .iter_errors(instance)
        .map(|error| format!("{error} at {}", error.instance_path()))
        .collect()
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
    assert_eq!(report["schema_version"], "cmdrvl.doctor.v1");
    assert_eq!(report["mode"], "health");
    assert_eq!(report["tool"], "vacuum");
    assert_eq!(report["tool_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(report["healthy"], true);
    assert!(report["findings"].is_array());
}

#[test]
fn doctor_capabilities_json_embeds_the_operator_contract() {
    let home = TempDir::new().expect("temp home should be created");
    let witness_path = home.path().join("witness.jsonl");
    let output = isolated_command(home.path(), &witness_path)
        .args(["doctor", "capabilities", "--json"])
        .output()
        .expect("vacuum doctor should run");

    assert_eq!(output.status.code(), Some(0));
    let report: Value =
        serde_json::from_slice(&output.stdout).expect("capabilities should emit JSON");

    assert_eq!(report["schema_version"], "cmdrvl.doctor.v1");
    assert_eq!(report["mode"], "capabilities");
    assert_eq!(report["contract"]["schema_version"], "operator.v1");
    assert_eq!(report["contract"]["name"], "vacuum");
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

    assert_eq!(report["schema_version"], "cmdrvl.doctor.v1");
    assert_eq!(report["mode"], "triage");
    assert_eq!(report["healthy"], true);
    assert!(report["next_actions"].is_array());
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
    assert_eq!(report["schema_version"], "cmdrvl.doctor.v1");
    assert_eq!(report["mode"], "triage");
    assert_eq!(report["healthy"], true);
    assert!(report["next_actions"].is_array());
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
    assert_eq!(report["schema_version"], "operator.v1");
    assert_eq!(report["commands"][0]["name"], "capabilities");
    assert_eq!(report["commands"][1]["name"], "robot-docs");
    assert_eq!(report["commands"][2]["name"], "doctor");
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
        stderr.contains("vacuum doctor triage --json"),
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
fn doctor_and_operator_reports_validate_against_spine_schemas() {
    let home = TempDir::new().expect("temp home should be created");
    let witness_path = home.path().join("witness.jsonl");

    for args in [
        ["doctor", "--json"].as_slice(),
        ["doctor", "capabilities", "--json"].as_slice(),
        ["doctor", "triage", "--json"].as_slice(),
    ] {
        let output = isolated_command(home.path(), &witness_path)
            .args(args)
            .output()
            .expect("doctor should run");
        assert_eq!(output.status.code(), Some(0), "{args:?}");
        let report = parse_stdout_json(&output);
        let errors = schema_errors(&report, DOCTOR_V1_SCHEMA);
        assert!(errors.is_empty(), "{args:?}: {errors:?}");
        if args[1] == "capabilities" {
            let contract_errors = schema_errors(&report["contract"], OPERATOR_V1_SCHEMA);
            assert!(
                contract_errors.is_empty(),
                "embedded contract: {contract_errors:?}"
            );
        }
    }

    let describe = isolated_command(home.path(), &witness_path)
        .arg("--describe")
        .output()
        .expect("describe should run");
    let describe_errors = schema_errors(&parse_stdout_json(&describe), OPERATOR_V1_SCHEMA);
    assert!(describe_errors.is_empty(), "describe: {describe_errors:?}");
}

#[test]
fn unhealthy_doctor_state_exits_one() {
    let home = TempDir::new().expect("temp home should be created");
    let witness_path = home.path().join("witness-directory");
    std::fs::create_dir_all(&witness_path).expect("witness directory should be created");
    let output = isolated_command(home.path(), &witness_path)
        .args(["doctor", "--json"])
        .output()
        .expect("unhealthy doctor should run");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let report = parse_stdout_json(&output);
    assert_eq!(report["schema_version"], "cmdrvl.doctor.v1");
    assert_eq!(report["healthy"], false);
    assert!(report["findings"].as_array().is_some_and(|findings| {
        findings.iter().any(|finding| {
            finding["id"] == "vacuum.witness_path_resolution" && finding["severity"] == "error"
        })
    }));
}

#[test]
fn no_color_and_non_tty_help_are_ansi_free_even_when_color_is_forced() {
    let home = TempDir::new().expect("temp home should be created");
    let witness_path = home.path().join("witness.jsonl");
    let output = isolated_command(home.path(), &witness_path)
        .env("CLICOLOR_FORCE", "1")
        .env_remove("NO_COLOR")
        .arg("--help")
        .output()
        .expect("help should run");

    assert_eq!(output.status.code(), Some(0));
    assert!(!output.stdout.windows(2).any(|window| window == b"\x1b["));
    assert!(!output.stderr.windows(2).any(|window| window == b"\x1b["));
}

#[test]
fn edit_distance_one_typos_name_the_correct_command_or_flag() {
    for (args, want) in [
        (["doctor", "healt"].as_slice(), "health"),
        (["robot-docs", "gude"].as_slice(), "guide"),
        (["--no-witnes"].as_slice(), "--no-witness"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
            .args(args)
            .output()
            .expect("typo invocation should run");
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(&format!("hint: did you mean `{want}`?")),
            "{args:?}: {stderr}"
        );
        assert!(
            output.stdout.is_empty(),
            "usage errors must not write stdout"
        );
    }
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
