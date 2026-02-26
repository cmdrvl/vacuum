use std::{fs, path::PathBuf, process::Command};

use serde_json::Value;
use tempfile::tempdir;
use vacuum::{
    record::{builder::VacuumRecord, mime::guess_from_extension},
    walk::filter::apply_filters,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn record(path: &str) -> VacuumRecord {
    let mut record = VacuumRecord::empty();
    record.relative_path = path.to_string();
    record
}

fn filter_matches(path: &str, include: &[&str], exclude: &[&str]) -> bool {
    let include = include
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    let exclude = exclude
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    !apply_filters(vec![record(path)], &include, &exclude).is_empty()
}

fn run_scan(paths: &[PathBuf]) -> Vec<Value> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vacuum"));
    command.arg("--no-witness");
    for path in paths {
        command.arg(path);
    }

    let output = command.output().expect("scan should execute");
    assert_eq!(output.status.code(), Some(0));
    String::from_utf8(output.stdout)
        .expect("stdout should be utf-8")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("line should be json"))
        .collect()
}

fn sorted_keys(rows: &[Value]) -> Vec<(String, String)> {
    rows.iter()
        .map(|row| {
            (
                row["relative_path"]
                    .as_str()
                    .expect("relative_path must be string")
                    .to_string(),
                row["root"]
                    .as_str()
                    .expect("root must be string")
                    .to_string(),
            )
        })
        .collect()
}

fn assert_sorted_contract(rows: &[Value]) {
    let keys = sorted_keys(rows);
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
}

fn run_refusal(args: &[&str]) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vacuum"));
    command.args(args).arg("--no-witness");
    let output = command.output().expect("command should execute");
    assert_eq!(output.status.code(), Some(2));
    serde_json::from_slice(&output.stdout).expect("refusal should be json")
}

macro_rules! mime_known_tests {
    ($($name:ident: $ext:expr => $mime:expr),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                assert_eq!(guess_from_extension(Some($ext)), Some($mime));
            }
        )+
    };
}

macro_rules! mime_unknown_tests {
    ($($name:ident: $ext:expr),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                assert_eq!(guess_from_extension(Some($ext)), None);
            }
        )+
    };
}

macro_rules! filter_case_tests {
    ($($name:ident: $path:expr, [$($inc:expr),*], [$($exc:expr),*], $expected:expr;)+) => {
        $(
            #[test]
            fn $name() {
                assert_eq!(filter_matches($path, &[$($inc),*], &[$($exc),*]), $expected);
            }
        )+
    };
}

mime_known_tests! {
    mime_lower_csv: ".csv" => "text/csv",
    mime_lower_tsv: ".tsv" => "text/tab-separated-values",
    mime_lower_txt: ".txt" => "text/plain",
    mime_lower_json: ".json" => "application/json",
    mime_lower_jsonl: ".jsonl" => "application/x-jsonlines",
    mime_lower_xml: ".xml" => "application/xml",
    mime_lower_pdf: ".pdf" => "application/pdf",
    mime_lower_xlsx: ".xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    mime_lower_xls: ".xls" => "application/vnd.ms-excel",
    mime_lower_parquet: ".parquet" => "application/vnd.apache.parquet",
    mime_lower_zip: ".zip" => "application/zip",
    mime_lower_gz: ".gz" => "application/gzip",
    mime_lower_yaml: ".yaml" => "application/x-yaml",
    mime_lower_yml: ".yml" => "application/x-yaml",
    mime_upper_csv: ".CSV" => "text/csv",
    mime_upper_tsv: ".TSV" => "text/tab-separated-values",
    mime_upper_txt: ".TXT" => "text/plain",
    mime_upper_json: ".JSON" => "application/json",
    mime_upper_jsonl: ".JSONL" => "application/x-jsonlines",
    mime_upper_xml: ".XML" => "application/xml",
    mime_upper_pdf: ".PDF" => "application/pdf",
    mime_upper_xlsx: ".XLSX" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    mime_upper_xls: ".XLS" => "application/vnd.ms-excel",
    mime_upper_parquet: ".PARQUET" => "application/vnd.apache.parquet",
    mime_upper_zip: ".ZIP" => "application/zip",
    mime_upper_gz: ".GZ" => "application/gzip",
    mime_upper_yaml: ".YAML" => "application/x-yaml",
    mime_upper_yml: ".YML" => "application/x-yaml",
    mime_mixed_csv: ".CsV" => "text/csv",
    mime_mixed_tsv: ".TsV" => "text/tab-separated-values",
    mime_mixed_txt: ".TxT" => "text/plain",
    mime_mixed_json: ".JsOn" => "application/json",
    mime_mixed_jsonl: ".JsOnL" => "application/x-jsonlines",
    mime_mixed_xml: ".XmL" => "application/xml",
    mime_mixed_pdf: ".PdF" => "application/pdf",
    mime_mixed_xlsx: ".XlSx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    mime_mixed_xls: ".XlS" => "application/vnd.ms-excel",
    mime_mixed_parquet: ".PaRqUeT" => "application/vnd.apache.parquet",
    mime_mixed_zip: ".ZiP" => "application/zip",
    mime_mixed_gz: ".Gz" => "application/gzip",
    mime_mixed_yaml: ".YaMl" => "application/x-yaml",
    mime_mixed_yml: ".YmL" => "application/x-yaml",
}

mime_unknown_tests! {
    mime_unknown_bin: ".bin",
    mime_unknown_exe: ".exe",
    mime_unknown_tmp: ".tmp",
    mime_unknown_foo: ".foo",
    mime_unknown_bar: ".bar",
    mime_unknown_baz: ".baz",
    mime_unknown_abc: ".abc",
    mime_unknown_123: ".123",
    mime_unknown_tar: ".tar",
    mime_unknown_md: ".md",
    mime_unknown_jpeg: ".jpeg",
    mime_unknown_png: ".png",
    mime_unknown_sqlite: ".sqlite",
    mime_unknown_env: ".env",
}

filter_case_tests! {
    filter_case_01: "alpha.csv", ["*.csv"], [], true;
    filter_case_02: "alpha.csv", ["*.txt"], [], false;
    filter_case_03: "nested/alpha.csv", ["**/*.csv"], [], true;
    filter_case_04: "nested/alpha.csv", ["*.csv"], [], false;
    filter_case_05: "a1/file.csv", ["a?/file.csv"], [], true;
    filter_case_06: "a11/file.csv", ["a?/file.csv"], [], false;
    filter_case_07: "a1/file.csv", ["[ab]1/file.csv"], [], true;
    filter_case_08: "c1/file.csv", ["[ab]1/file.csv"], [], false;
    filter_case_09: "report.json", ["*.json", "*.csv"], [], true;
    filter_case_10: "report.txt", ["*.json", "*.csv"], [], false;
    filter_case_11: "dir\\leaf.csv", ["dir/*.csv"], [], true;
    filter_case_12: "deep/tree/leaf.yaml", ["**/*.yaml"], [], true;
    filter_case_13: "deep/tree/leaf.yaml", ["*.yaml"], [], false;
    filter_case_14: "mix/data.parquet", ["mix/*.parquet"], [], true;
    filter_case_15: "mix/data.parquet", ["mix/*.csv"], [], false;
    filter_case_16: "alpha.csv", ["*.csv"], ["alpha.csv"], false;
    filter_case_17: "alpha.csv", ["*.csv"], ["beta.csv"], true;
    filter_case_18: "nested/alpha.csv", ["**/*.csv"], ["nested/*.csv"], false;
    filter_case_19: "nested/alpha.csv", ["**/*.csv"], ["other/*.csv"], true;
    filter_case_20: "a1/file.csv", ["a?/file.csv"], ["a1/file.csv"], false;
    filter_case_21: "a1/file.csv", ["a?/file.csv"], ["a2/file.csv"], true;
    filter_case_22: "report.json", ["*.json", "*.csv"], ["report.json"], false;
    filter_case_23: "report.csv", ["*.json", "*.csv"], ["report.json"], true;
    filter_case_24: "deep/tree/leaf.yaml", ["**/*.yaml"], ["deep/**"], false;
    filter_case_25: "deep/tree/leaf.yaml", ["**/*.yaml"], ["other/**"], true;
    filter_case_26: "nested\\inner\\leaf.csv", ["nested/**/*.csv"], ["nested/**/leaf.csv"], false;
    filter_case_27: "nested\\inner\\leaf.csv", ["nested/**/*.csv"], ["nested/**/other.csv"], true;
    filter_case_28: "x/file.csv", [], ["x/*"], false;
    filter_case_29: "x/file.csv", [], ["y/*"], true;
    filter_case_30: "x/file.csv", [], [], true;
}

#[test]
fn sort_contract_nested_fixture() {
    let rows = run_scan(&[fixture("nested")]);
    assert_sorted_contract(&rows);
}

#[test]
fn sort_contract_simple_fixture() {
    let rows = run_scan(&[fixture("simple")]);
    assert_sorted_contract(&rows);
}

#[test]
fn sort_contract_multi_root_fixture() {
    let rows = run_scan(&[fixture("nested"), fixture("simple")]);
    assert_sorted_contract(&rows);
}

#[test]
fn sort_contract_is_stable_across_runs_for_multi_root() {
    let first = run_scan(&[fixture("nested"), fixture("simple")]);
    let second = run_scan(&[fixture("nested"), fixture("simple")]);
    assert_eq!(sorted_keys(&first), sorted_keys(&second));
}

#[test]
fn collision_two_roots_same_relative_path_both_records_present() {
    let temp_dir = tempdir().expect("tempdir should be created");
    let root_a = temp_dir.path().join("a");
    let root_b = temp_dir.path().join("b");
    fs::create_dir_all(&root_a).expect("root a should exist");
    fs::create_dir_all(&root_b).expect("root b should exist");
    fs::write(root_a.join("shared.csv"), "a").expect("file a should exist");
    fs::write(root_b.join("shared.csv"), "b").expect("file b should exist");

    let rows = run_scan(&[root_a.clone(), root_b.clone()]);
    let shared = rows
        .iter()
        .filter(|row| row["relative_path"] == "shared.csv")
        .collect::<Vec<_>>();
    assert_eq!(shared.len(), 2);
    assert_eq!(shared[0]["root"], root_a.to_string_lossy().to_string());
    assert_eq!(shared[1]["root"], root_b.to_string_lossy().to_string());
}

#[test]
fn collision_three_roots_tie_breaks_by_root_ordering() {
    let temp_dir = tempdir().expect("tempdir should be created");
    let root_a = temp_dir.path().join("a");
    let root_b = temp_dir.path().join("b");
    let root_c = temp_dir.path().join("c");
    fs::create_dir_all(&root_a).expect("root a should exist");
    fs::create_dir_all(&root_b).expect("root b should exist");
    fs::create_dir_all(&root_c).expect("root c should exist");
    fs::write(root_a.join("shared.csv"), "a").expect("file a should exist");
    fs::write(root_b.join("shared.csv"), "b").expect("file b should exist");
    fs::write(root_c.join("shared.csv"), "c").expect("file c should exist");

    let rows = run_scan(&[root_c.clone(), root_a.clone(), root_b.clone()]);
    let shared_roots = rows
        .iter()
        .filter(|row| row["relative_path"] == "shared.csv")
        .map(|row| row["root"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();

    assert_eq!(
        shared_roots,
        vec![
            root_a.to_string_lossy().to_string(),
            root_b.to_string_lossy().to_string(),
            root_c.to_string_lossy().to_string()
        ]
    );
}

#[test]
fn sort_contract_preserves_byte_order_for_symbol_and_alnum_names() {
    let temp_dir = tempdir().expect("tempdir should be created");
    let root = temp_dir.path().join("symbol-order");
    fs::create_dir_all(&root).expect("root should exist");
    fs::write(root.join("!a.txt"), "!").expect("symbol file should exist");
    fs::write(root.join("0a.txt"), "0").expect("numeric file should exist");
    fs::write(root.join("a.txt"), "a").expect("alpha file should exist");

    let rows = run_scan(&[root]);
    let names = rows
        .iter()
        .map(|row| {
            row["relative_path"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["!a.txt", "0a.txt", "a.txt"]);
}

#[test]
fn sort_contract_is_lexicographic_for_numeric_suffixes() {
    let temp_dir = tempdir().expect("tempdir should be created");
    let root = temp_dir.path().join("numeric");
    fs::create_dir_all(&root).expect("root should exist");
    fs::write(root.join("file10.txt"), "10").expect("file10 should exist");
    fs::write(root.join("file2.txt"), "2").expect("file2 should exist");

    let rows = run_scan(&[root]);
    let names = rows
        .iter()
        .map(|row| {
            row["relative_path"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["file10.txt", "file2.txt"]);
}

#[test]
fn skipped_record_has_null_size_and_mtime() {
    let rows = run_scan(&[fixture("symlinks")]);
    let skipped = rows
        .iter()
        .find(|row| row["relative_path"] == "broken_link")
        .expect("broken link should be present");
    assert_eq!(skipped["_skipped"], true);
    assert!(skipped["size"].is_null());
    assert!(skipped["mtime"].is_null());
}

#[test]
fn skipped_record_exposes_warning_payload_shape() {
    let rows = run_scan(&[fixture("symlinks")]);
    let skipped = rows
        .iter()
        .find(|row| row["relative_path"] == "broken_link")
        .expect("broken link should be present");
    assert_eq!(skipped["_warnings"][0]["tool"], "vacuum");
    assert_eq!(skipped["_warnings"][0]["code"], "E_IO");
    assert!(skipped["_warnings"][0]["message"].is_string());
    assert!(skipped["_warnings"][0]["detail"]["error"].is_string());
}

#[test]
fn skipped_record_keeps_derivable_extension_and_mime() {
    let temp_dir = tempdir().expect("tempdir should be created");
    let root = temp_dir.path().join("broken");
    fs::create_dir_all(&root).expect("root should exist");
    #[cfg(unix)]
    std::os::unix::fs::symlink("missing.csv", root.join("missing.csv"))
        .expect("symlink should be created");
    #[cfg(windows)]
    std::os::windows::fs::symlink_file("missing.csv", root.join("missing.csv"))
        .expect("symlink should be created");

    let rows = run_scan(&[root]);
    let skipped = rows
        .iter()
        .find(|row| row["relative_path"] == "missing.csv")
        .expect("missing csv should be present");
    assert_eq!(skipped["_skipped"], true);
    assert_eq!(skipped["extension"], ".csv");
    assert_eq!(skipped["mime_guess"], "text/csv");
}

#[test]
fn refusal_envelope_for_missing_root_has_expected_code_and_message() {
    let refusal = run_refusal(&["/definitely-missing-vacuum-root-parity"]);
    assert_eq!(refusal["version"], "vacuum.v0");
    assert_eq!(refusal["outcome"], "REFUSAL");
    assert_eq!(refusal["refusal"]["code"], "E_ROOT_NOT_FOUND");
    assert_eq!(refusal["refusal"]["message"], "Root path does not exist");
}

#[test]
fn refusal_envelope_for_missing_root_has_null_next_command() {
    let refusal = run_refusal(&["/definitely-missing-vacuum-root-parity-next"]);
    assert!(refusal["refusal"]["next_command"].is_null());
}

#[test]
fn refusal_envelope_for_non_directory_root_uses_e_io() {
    let temp_dir = tempdir().expect("tempdir should be created");
    let file = temp_dir.path().join("not-dir.txt");
    fs::write(&file, "x").expect("file should exist");

    let refusal = run_refusal(&[file.to_string_lossy().as_ref()]);
    assert_eq!(refusal["refusal"]["code"], "E_IO");
    assert_eq!(
        refusal["refusal"]["message"],
        "Filesystem error during scan"
    );
}

#[test]
fn refusal_envelope_for_empty_roots_uses_not_found_shape() {
    let refusal = run_refusal(&[]);
    assert_eq!(refusal["refusal"]["code"], "E_ROOT_NOT_FOUND");
    assert_eq!(refusal["refusal"]["detail"]["root"], "");
}

#[cfg(unix)]
#[test]
fn refusal_envelope_for_unreadable_root_uses_permission_code() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = tempdir().expect("tempdir should be created");
    let root = temp_dir.path().join("restricted");
    fs::create_dir_all(&root).expect("root should exist");

    let original_permissions = fs::metadata(&root)
        .expect("metadata should exist")
        .permissions();
    let mut restricted = original_permissions.clone();
    restricted.set_mode(0o000);
    fs::set_permissions(&root, restricted).expect("permissions should be set");

    let refusal = run_refusal(&[root.to_string_lossy().as_ref()]);

    let mut restored = original_permissions;
    restored.set_mode(0o755);
    fs::set_permissions(&root, restored).expect("permissions should be restored");

    assert_eq!(refusal["refusal"]["code"], "E_ROOT_PERMISSION");
}
