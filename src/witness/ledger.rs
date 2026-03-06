use std::{
    env,
    ffi::OsString,
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
    resolve_ledger_path_from_env(|key| env::var_os(key))
}

fn resolve_ledger_path_from_env<F>(get_env: F) -> PathBuf
where
    F: Fn(&str) -> Option<OsString>,
{
    if let Some(path) = get_env("EPISTEMIC_WITNESS")
        && !path.is_empty()
    {
        return PathBuf::from(path);
    }

    if let Some(home) = get_env("HOME")
        .or_else(|| get_env("USERPROFILE"))
        .filter(|value| !value.is_empty())
    {
        return PathBuf::from(home).join(".epistemic").join("witness.jsonl");
    }

    PathBuf::from(".epistemic").join("witness.jsonl")
}

#[cfg(test)]
mod tests {
    use super::resolve_ledger_path_from_env;
    use std::ffi::OsString;
    use std::path::PathBuf;

    #[test]
    fn empty_epistemic_witness_falls_back_to_home() {
        let path = resolve_ledger_path_from_env(|key| match key {
            "EPISTEMIC_WITNESS" => Some(OsString::new()),
            "HOME" => Some(OsString::from("/tmp/home")),
            _ => None,
        });

        assert_eq!(path, PathBuf::from("/tmp/home/.epistemic/witness.jsonl"));
    }

    #[test]
    fn empty_home_falls_back_to_repo_epistemic_dir() {
        let path = resolve_ledger_path_from_env(|key| match key {
            "EPISTEMIC_WITNESS" => None,
            "HOME" => Some(OsString::new()),
            "USERPROFILE" => Some(OsString::new()),
            _ => None,
        });

        assert_eq!(path, PathBuf::from(".epistemic/witness.jsonl"));
    }
}
