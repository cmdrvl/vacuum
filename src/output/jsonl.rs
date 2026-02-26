use crate::record::builder::VacuumRecord;

pub fn emit_records(records: &[VacuumRecord]) {
    for line in serialize_sorted_jsonl(records) {
        println!("{line}");
    }
}

pub fn operator_manifest() -> &'static str {
    include_str!("../../operator.json")
}

pub fn print_operator_manifest() {
    println!("{}", operator_manifest().trim_end());
}

pub fn print_schema_stub() {
    println!(
        "{{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"title\":\"vacuum.v0\"}}"
    );
}

fn serialize_sorted_jsonl(records: &[VacuumRecord]) -> Vec<String> {
    sorted_records(records)
        .into_iter()
        .filter_map(|record| serde_json::to_string(&record).ok())
        .collect()
}

fn sorted_records(records: &[VacuumRecord]) -> Vec<VacuumRecord> {
    let mut sorted = records.to_vec();
    sorted.sort_by(|left, right| {
        left.relative_path
            .cmp(&right.relative_path)
            .then_with(|| left.root.cmp(&right.root))
    });
    sorted
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use crate::record::builder::VacuumRecord;

    use super::{operator_manifest, serialize_sorted_jsonl, sorted_records};

    fn record(relative_path: &str, root: &str) -> VacuumRecord {
        let mut record = VacuumRecord::empty();
        record.path = format!("{root}/{relative_path}");
        record.relative_path = relative_path.to_string();
        record.root = root.to_string();
        record.size = Some(1);
        record.mtime = Some("2026-01-01T00:00:00.000Z".to_string());
        record.extension = Some(".csv".to_string());
        record.mime_guess = Some("text/csv".to_string());
        record
    }

    #[test]
    fn operator_manifest_is_valid_json() {
        let manifest: Value =
            serde_json::from_str(operator_manifest()).expect("operator manifest should parse");

        assert_eq!(manifest["schema_version"], "operator.v0");
        assert_eq!(manifest["name"], "vacuum");
        assert_eq!(manifest["exit_codes"]["0"]["meaning"], "SCAN_COMPLETE");
        assert_eq!(manifest["exit_codes"]["2"]["meaning"], "REFUSAL");
        assert_eq!(manifest["pipeline"]["upstream"], serde_json::json!([]));
        assert_eq!(
            manifest["pipeline"]["downstream"],
            serde_json::json!(["hash"])
        );
    }

    #[test]
    fn records_are_sorted_by_relative_path_then_root() {
        let records = vec![
            record("b.csv", "/root-b"),
            record("a.csv", "/root-z"),
            record("a.csv", "/root-a"),
        ];

        let sorted = sorted_records(&records);
        let keys = sorted
            .iter()
            .map(|record| (record.relative_path.as_str(), record.root.as_str()))
            .collect::<Vec<_>>();

        assert_eq!(
            keys,
            vec![
                ("a.csv", "/root-a"),
                ("a.csv", "/root-z"),
                ("b.csv", "/root-b")
            ]
        );
    }

    #[test]
    fn jsonl_emission_is_one_json_record_per_line() {
        let records = vec![record("b.csv", "/root-b"), record("a.csv", "/root-a")];
        let lines = serialize_sorted_jsonl(&records);

        assert_eq!(lines.len(), 2);
        let parsed = lines
            .iter()
            .map(|line| serde_json::from_str::<Value>(line).expect("json line should parse"))
            .collect::<Vec<_>>();

        assert_eq!(parsed[0]["relative_path"], "a.csv");
        assert_eq!(parsed[1]["relative_path"], "b.csv");
        assert_eq!(parsed[0]["version"], "vacuum.v0");
        assert_eq!(
            parsed[1]["tool_versions"],
            json!({ "vacuum": env!("CARGO_PKG_VERSION") })
        );
    }
}
