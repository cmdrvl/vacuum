use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VacuumRecord {
    pub version: &'static str,
    pub path: String,
    pub relative_path: String,
    pub root: String,
    pub size: Option<u64>,
    pub mtime: Option<String>,
    pub extension: Option<String>,
    pub mime_guess: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _skipped: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _warnings: Option<Vec<Warning>>,
    pub tool_versions: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Warning {
    pub tool: String,
    pub code: String,
    pub message: String,
    pub detail: Value,
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
            _skipped: None,
            _warnings: None,
            tool_versions,
        }
    }
}
