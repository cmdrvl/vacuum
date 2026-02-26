use crate::record::builder::VacuumRecord;

pub fn emit_records(_records: &[VacuumRecord]) {}

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

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::operator_manifest;

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
}
