use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

use crate::witness::record::WitnessRecord;

pub fn append(record: &WitnessRecord) -> std::io::Result<()> {
    let path = resolve_ledger_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let encoded = serde_json::to_string(record).map_err(std::io::Error::other)?;
    writeln!(file, "{encoded}")?;
    Ok(())
}

pub fn resolve_ledger_path() -> PathBuf {
    if let Some(path) = env::var_os("EPISTEMIC_WITNESS") {
        return PathBuf::from(path);
    }

    if let Some(home) = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE")) {
        return PathBuf::from(home).join(".epistemic").join("witness.jsonl");
    }

    PathBuf::from(".epistemic").join("witness.jsonl")
}
