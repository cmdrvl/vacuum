use std::process::Command;

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn progress_emits_structured_stderr_without_polluting_stdout_manifest() {
    let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .arg(fixture("simple"))
        .arg("--progress")
        .output()
        .expect("vacuum binary should run");

    assert!(output.status.success(), "scan should exit 0");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");

    for line in stdout.lines() {
        let record: serde_json::Value =
            serde_json::from_str(line).expect("stdout should remain JSONL manifest records");
        assert_eq!(record["version"], "vacuum.v0");
    }

    let progress_lines = stderr
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|value| value["type"] == "progress")
        .collect::<Vec<_>>();
    assert!(
        !progress_lines.is_empty(),
        "progress should emit structured progress JSONL to stderr"
    );
}

#[test]
fn progress_emits_structured_warning_records_for_skipped_entries() {
    let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .arg(fixture("symlinks"))
        .arg("--progress")
        .output()
        .expect("vacuum binary should run");

    assert!(output.status.success(), "scan should exit 0");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    let warning_lines = stderr
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|value| value["type"] == "warning")
        .collect::<Vec<_>>();

    assert!(
        !warning_lines.is_empty(),
        "skipped entries should emit structured warning lines with --progress"
    );
}

#[test]
fn without_progress_stderr_warnings_are_unstructured_lines() {
    let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .arg(fixture("symlinks"))
        .output()
        .expect("vacuum binary should run");

    assert!(output.status.success(), "scan should exit 0");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(
        stderr
            .lines()
            .any(|line| line.starts_with("vacuum: skipped ")),
        "without --progress warnings should be unstructured text lines"
    );
}
