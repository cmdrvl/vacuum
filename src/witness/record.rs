use std::path::PathBuf;

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WitnessInput {
    pub path: String,
    #[serde(default)]
    pub hash: Option<String>,
    #[serde(default)]
    pub bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WitnessRecord {
    #[serde(default)]
    pub id: String,
    pub tool: String,
    pub version: String,
    #[serde(default)]
    pub binary_hash: String,
    pub inputs: Vec<WitnessInput>,
    pub params: serde_json::Value,
    pub outcome: String,
    pub exit_code: u8,
    pub output_hash: String,
    #[serde(default)]
    pub prev: Option<String>,
    pub ts: String,
}

impl WitnessRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn from_run(
        roots: &[PathBuf],
        include: &[String],
        exclude: &[String],
        no_follow: bool,
        outcome: impl Into<String>,
        exit_code: u8,
        output_hash: String,
        prev: Option<String>,
    ) -> Self {
        let inputs = roots
            .iter()
            .map(|root| WitnessInput {
                path: root.to_string_lossy().into_owned(),
                hash: None,
                bytes: None,
            })
            .collect::<Vec<_>>();

        Self {
            id: String::new(),
            tool: "vacuum".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            binary_hash: hash_self()
                .map(|value| format!("blake3:{value}"))
                .unwrap_or_default(),
            inputs,
            params: serde_json::json!({
                "roots": roots
                    .iter()
                    .map(|root| root.to_string_lossy().into_owned())
                    .collect::<Vec<_>>(),
                "include": include,
                "exclude": exclude,
                "no_follow": no_follow,
            }),
            outcome: outcome.into(),
            exit_code,
            output_hash,
            prev,
            ts: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        }
    }

    pub fn compute_id(&mut self) {
        self.id.clear();
        self.id = format!(
            "blake3:{}",
            blake3::hash(canonical_json(self).as_bytes()).to_hex()
        );
    }
}

pub fn canonical_json(record: &WitnessRecord) -> String {
    let value = serde_json::to_value(record).expect("WitnessRecord should serialize");
    serde_json::to_string(&value).expect("WitnessRecord JSON should encode")
}

fn hash_self() -> Result<String, std::io::Error> {
    let path = std::env::current_exe()?;
    let bytes = std::fs::read(path)?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}
