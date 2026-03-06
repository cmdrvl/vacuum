#[cfg(unix)]
mod tests {
    use std::{fs, path::PathBuf, process::Command};

    use serde_json::Value;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }

    #[test]
    fn progress_mode_emits_structured_stderr_without_polluting_stdout_manifest() {
        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        let witness_path = temp_dir.path().join("witness.jsonl");
        let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
            .arg(fixture("symlinks"))
            .arg("--progress")
            .env("EPISTEMIC_WITNESS", &witness_path)
            .output()
            .expect("vacuum binary should run");

        assert!(output.status.success(), "scan should exit 0");

        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");

        let manifest_records = stdout
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).expect("manifest line should be json"))
            .collect::<Vec<_>>();
        assert!(
            !manifest_records.is_empty(),
            "scan should emit manifest records to stdout"
        );
        assert!(
            manifest_records
                .iter()
                .all(|value| value["version"] == "vacuum.v0")
        );
        assert!(manifest_records.iter().all(|value| value["type"].is_null()));

        let progress_lines = stderr
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).expect("progress line should be json"))
            .collect::<Vec<_>>();
        assert!(
            progress_lines.iter().any(|line| line["type"] == "progress"),
            "stderr should contain structured progress records"
        );
        assert!(
            progress_lines.iter().any(|line| line["type"] == "warning"),
            "stderr should contain structured warning records for skipped files"
        );
    }

    #[test]
    fn progress_mode_keeps_witness_append_failures_structured() {
        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        let witness_parent = temp_dir.path().join("blocked-parent");
        fs::write(&witness_parent, "not a directory").expect("blocker file should be created");
        let witness_path = witness_parent.join("witness.jsonl");

        let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
            .arg(fixture("symlinks"))
            .arg("--progress")
            .env("EPISTEMIC_WITNESS", &witness_path)
            .output()
            .expect("vacuum binary should run");

        assert!(output.status.success(), "scan should exit 0");

        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");

        assert!(
            stdout
                .lines()
                .all(|line| serde_json::from_str::<Value>(line).is_ok()),
            "stdout should remain valid manifest jsonl"
        );

        let stderr_lines = stderr
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).expect("progress line should be json"))
            .collect::<Vec<_>>();
        assert!(
            stderr_lines.iter().any(|line| {
                line["type"] == "warning"
                    && line["message"]
                        .as_str()
                        .is_some_and(|message| message.contains("Witness append failed"))
            }),
            "witness append failures should stay structured in progress mode"
        );
    }

    #[test]
    fn default_mode_emits_unstructured_warnings_only() {
        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        let witness_path = temp_dir.path().join("witness.jsonl");
        let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
            .arg(fixture("symlinks"))
            .env("EPISTEMIC_WITNESS", &witness_path)
            .output()
            .expect("vacuum binary should run");

        assert!(output.status.success(), "scan should exit 0");

        let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
        assert!(
            stderr.lines().any(|line| line.contains("vacuum: skipped")),
            "default stderr should include unstructured warning lines"
        );
        assert!(
            stderr
                .lines()
                .all(|line| serde_json::from_str::<Value>(line).is_err()),
            "default stderr should not emit structured json progress lines"
        );
    }
}
