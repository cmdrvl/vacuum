use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VacuumRecord {
    pub version: &'static str,
    pub path: String,
    pub relative_path: String,
    pub root: String,
    pub size: Option<u64>,
    pub mtime: Option<String>,
    pub extension: Option<String>,
    pub mime_guess: Option<String>,
    pub tool_versions: BTreeMap<String, String>,
}

impl VacuumRecord {
    pub fn empty() -> Self {
        let mut tool_versions = BTreeMap::new();
        tool_versions.insert("vacuum".to_string(), env!("CARGO_PKG_VERSION").to_string());

        Self {
            version: "vacuum.v0",
            path: String::new(),
            relative_path: String::new(),
            root: String::new(),
            size: None,
            mtime: None,
            extension: None,
            mime_guess: None,
            tool_versions,
        }
    }
}
