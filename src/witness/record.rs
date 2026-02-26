use chrono::{SecondsFormat, Utc};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WitnessRecord {
    pub version: &'static str,
    pub tool: String,
    pub outcome: String,
    pub exit_code: u8,
    pub ts: String,
}

impl WitnessRecord {
    pub fn new(outcome: impl Into<String>, exit_code: u8) -> Self {
        Self {
            version: "witness.v0",
            tool: "vacuum".to_string(),
            outcome: outcome.into(),
            exit_code,
            ts: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}
