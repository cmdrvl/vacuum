use std::path::PathBuf;

use crate::record::builder::VacuumRecord;

pub fn scan_roots(_roots: &[PathBuf], _follow_symlinks: bool) -> Vec<VacuumRecord> {
    Vec::new()
}
