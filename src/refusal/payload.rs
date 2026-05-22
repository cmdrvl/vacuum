use std::path::Path;

use serde::Serialize;
use serde_json::{Value, json};

use crate::refusal::codes::RefusalCode;

#[derive(Debug, Clone)]
pub struct Refusal {
    pub code: RefusalCode,
    pub detail: Value,
    pub next_command: Option<String>,
}

impl Refusal {
    pub fn new(code: RefusalCode, detail: Value) -> Self {
        Self {
            code,
            detail,
            next_command: None,
        }
    }

    pub fn with_next_command(mut self, next_command: impl Into<String>) -> Self {
        self.next_command = Some(next_command.into());
        self
    }
}

#[derive(Serialize)]
struct RefusalEnvelope<'a> {
    version: &'static str,
    outcome: &'static str,
    refusal: RefusalBody<'a>,
}

#[derive(Serialize)]
struct RefusalBody<'a> {
    code: &'static str,
    message: &'static str,
    detail: &'a Value,
    next_command: Option<String>,
}

pub fn emit(refusal: &Refusal) {
    println!("{}", render(refusal));
}

pub fn render(refusal: &Refusal) -> String {
    let envelope = RefusalEnvelope {
        version: "vacuum.v0",
        outcome: "REFUSAL",
        refusal: RefusalBody {
            code: refusal.code.as_str(),
            message: refusal.code.message(),
            detail: &refusal.detail,
            next_command: refusal.next_command.clone(),
        },
    };

    match serde_json::to_string(&envelope) {
        Ok(encoded) => encoded,
        Err(error) => {
            eprintln!("vacuum: failed to encode refusal envelope: {error}");
            json!({
                "version": "vacuum.v0",
                "outcome": "REFUSAL",
                "refusal": {
                    "code": RefusalCode::Io.as_str(),
                    "message": RefusalCode::Io.message(),
                    "detail": {"error": "refusal_encoding_failure"},
                    "next_command": null
                }
            })
            .to_string()
        }
    }
}

pub fn empty_roots_refusal() -> Refusal {
    Refusal::new(RefusalCode::RootNotFound, json!({ "root": "" }))
}

pub fn guard_preflight_refusal(settings_path: Option<&Path>, findings: Vec<String>) -> Refusal {
    Refusal::new(
        RefusalCode::GuardPreflight,
        json!({
            "settings_path": settings_path.map(|path| path.display().to_string()),
            "required": {
                "veil": {
                    "event": "PreToolUse",
                    "matchers": ["Read", "Grep", "Bash"]
                },
                "dcg": {
                    "event": "PreToolUse",
                    "matchers": ["Bash"]
                }
            },
            "findings": findings,
        }),
    )
    .with_next_command("Install or repair veil and dcg Claude PreToolUse hooks, then rerun vacuum")
}
