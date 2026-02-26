use crate::record::builder::VacuumRecord;

pub fn apply_filters(
    records: Vec<VacuumRecord>,
    _include: &[String],
    _exclude: &[String],
) -> Vec<VacuumRecord> {
    records
}
