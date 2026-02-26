use serde::Serialize;
use serde_json::{Value, json};

use crate::refusal::codes::RefusalCode;

#[derive(Debug, Clone)]
pub struct Refusal {
    pub code: RefusalCode,
    pub detail: Value,
}

impl Refusal {
    pub fn new(code: RefusalCode, detail: Value) -> Self {
        Self { code, detail }
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
    let envelope = RefusalEnvelope {
        version: "vacuum.v0",
        outcome: "REFUSAL",
        refusal: RefusalBody {
            code: refusal.code.as_str(),
            message: refusal.code.message(),
            detail: &refusal.detail,
            next_command: None,
        },
    };

    match serde_json::to_string(&envelope) {
        Ok(encoded) => println!("{encoded}"),
        Err(error) => {
            eprintln!("vacuum: failed to encode refusal envelope: {error}");
            let fallback = json!({
                "version": "vacuum.v0",
                "outcome": "REFUSAL",
                "refusal": {
                    "code": RefusalCode::Io.as_str(),
                    "message": RefusalCode::Io.message(),
                    "detail": {"error": "refusal_encoding_failure"},
                    "next_command": null
                }
            });
            println!("{fallback}");
        }
    }
}

pub fn empty_roots_refusal() -> Refusal {
    Refusal::new(RefusalCode::RootNotFound, json!({ "root": "" }))
}
