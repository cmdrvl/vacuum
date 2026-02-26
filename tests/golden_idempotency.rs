use std::{fs, path::PathBuf, process::Command};

use serde_json::Value;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn golden(name: &str) -> Vec<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(name);
    let content = fs::read_to_string(path).expect("golden file should be readable");
    serde_json::from_str(&content).expect("golden file should be valid json")
}

fn run_scan(args: &[&str]) -> Vec<Value> {
    let output = Command::new(env!("CARGO_BIN_EXE_vacuum"))
        .args(args)
        .output()
        .expect("vacuum binary should run");
    assert_eq!(output.status.code(), Some(0));

    String::from_utf8(output.stdout)
        .expect("stdout should be utf-8")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("line should be json"))
        .collect()
}

fn normalize(records: &[Value]) -> Vec<Value> {
    records
        .iter()
        .map(|record| {
            serde_json::json!({
                "relative_path": record["relative_path"],
                "size": record["size"],
                "extension": record["extension"],
                "mime_guess": record["mime_guess"],
                "_skipped": record["_skipped"].as_bool().unwrap_or(false)
            })
        })
        .collect()
}

#[test]
fn golden_baseline_include_and_exclude_outputs_match_structural_expectations() {
    let baseline = run_scan(&[fixture("simple").to_string_lossy().as_ref()]);
    assert_eq!(normalize(&baseline), golden("simple_baseline.json"));

    let include = run_scan(&[
        fixture("simple").to_string_lossy().as_ref(),
        "--include",
        "*.csv",
    ]);
    assert_eq!(normalize(&include), golden("simple_include_csv.json"));

    let exclude = run_scan(&[
        fixture("simple").to_string_lossy().as_ref(),
        "--exclude",
        "*.csv",
    ]);
    assert_eq!(normalize(&exclude), golden("simple_exclude_csv.json"));
}

#[test]
fn idempotent_scans_produce_equivalent_ordered_record_sets() {
    let first = run_scan(&[fixture("nested").to_string_lossy().as_ref()]);
    let second = run_scan(&[fixture("nested").to_string_lossy().as_ref()]);

    assert_eq!(normalize(&first), normalize(&second));
}

#[test]
fn ordering_is_stable_by_relative_path_then_root() {
    let records = run_scan(&[
        fixture("nested").to_string_lossy().as_ref(),
        fixture("simple").to_string_lossy().as_ref(),
    ]);

    let keys = records
        .iter()
        .map(|record| {
            (
                record["relative_path"]
                    .as_str()
                    .expect("relative_path should be string")
                    .to_string(),
                record["root"]
                    .as_str()
                    .expect("root should be string")
                    .to_string(),
            )
        })
        .collect::<Vec<_>>();

    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
}
