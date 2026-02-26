#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessRecord {
    pub tool: String,
    pub outcome: String,
    pub exit_code: u8,
}

impl WitnessRecord {
    pub fn new(outcome: impl Into<String>, exit_code: u8) -> Self {
        Self {
            tool: "vacuum".to_string(),
            outcome: outcome.into(),
            exit_code,
        }
    }
}
