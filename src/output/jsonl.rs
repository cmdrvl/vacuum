use crate::record::builder::VacuumRecord;

pub fn emit_records(_records: &[VacuumRecord]) {}

pub fn print_operator_stub() {
    println!(
        "{{\"schema_version\":\"operator.v0\",\"name\":\"vacuum\",\"version\":\"{}\"}}",
        env!("CARGO_PKG_VERSION")
    );
}

pub fn print_schema_stub() {
    println!(
        "{{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"title\":\"vacuum.v0\"}}"
    );
}
