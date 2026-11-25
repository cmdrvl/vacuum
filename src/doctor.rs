use serde_json::{Value, json};
use std::fs;

use crate::{
    cli::{args::DoctorAction, exit},
    output::jsonl::schema_manifest,
    witness::ledger::resolve_ledger_path,
};

const DOCTOR_SCHEMA_VERSION: &str = "cmdrvl.doctor.v1";
const OPERATOR_SCHEMA_VERSION: &str = "operator.v1";
const CONTRACT_VERSION: &str = "1.0.0";
const OPERATOR_MANIFEST: &str = include_str!("../operator.json");
const HEALTH_SCHEMA_VERSION: &str = "vacuum.doctor.health.v1";
const CAPABILITIES_SCHEMA_VERSION: &str = "vacuum.doctor.capabilities.v1";
const TRIAGE_SCHEMA_VERSION: &str = "vacuum.doctor.triage.v1";
const READ_ONLY_DOCTOR_CONTRACT: &str = "cmdrvl.read_only_doctor.v1";

pub fn dispatch(robot_triage: bool, json_mode: bool, action: Option<&DoctorAction>) -> u8 {
    if robot_triage {
        return dispatch_robot_triage();
    }

    match action {
        Some(DoctorAction::Health { json }) => render_health(*json || json_mode),
        Some(DoctorAction::Capabilities { json }) => render_capabilities(*json || json_mode),
        Some(DoctorAction::Triage { json }) => render_triage(*json || json_mode),
        Some(DoctorAction::RobotDocs) => dispatch_robot_docs(),
        None => render_health(json_mode),
    }
}

pub fn dispatch_unavailable_fix() -> u8 {
    eprintln!(
        "vacuum doctor --fix is not available: no fixer has detector, backup, inverse, and fixture coverage yet."
    );
    eprintln!("Safe alternatives:");
    eprintln!("  vacuum doctor triage --json");
    eprintln!("  vacuum doctor capabilities --json");
    eprintln!("  vacuum capabilities --json");
    exit::REFUSAL
}

pub fn dispatch_capabilities(json_mode: bool) -> u8 {
    if json_mode {
        print_operator_json();
    } else {
        println!("vacuum capabilities");
        println!("operator contract: vacuum --describe");
        println!("doctor: vacuum doctor --json");
        println!("robot docs: vacuum robot-docs guide");
    }
    exit::SCAN_COMPLETE
}

pub fn dispatch_robot_docs() -> u8 {
    print_robot_docs();
    exit::SCAN_COMPLETE
}

pub fn dispatch_robot_triage() -> u8 {
    let report = doctor_v1_report("triage");
    print_json(&report);
    exit_for_report(&report)
}

fn render_health(json_mode: bool) -> u8 {
    let report = doctor_v1_report("health");
    if json_mode {
        print_json(&report);
    } else {
        print_health_human(&report);
    }
    exit_for_report(&report)
}

fn render_capabilities(json_mode: bool) -> u8 {
    let report = doctor_v1_report("capabilities");
    if json_mode {
        print_json(&report);
    } else {
        print_capabilities_human(&report);
    }
    exit_for_report(&report)
}

fn render_triage(json_mode: bool) -> u8 {
    let report = doctor_v1_report("triage");
    if json_mode {
        print_json(&report);
    } else {
        print_health_human(&report);
        println!("next: vacuum capabilities --json");
    }
    exit_for_report(&report)
}

fn health_report() -> Value {
    let checks = vec![
        operator_manifest_check(),
        schema_manifest_check(),
        witness_path_check(),
        scan_stdout_contract_check(),
    ];
    let summary = summary_from_checks(&checks);
    let ok = summary.get("error").and_then(Value::as_u64).unwrap_or(0) == 0;

    json!({
        "schema_version": HEALTH_SCHEMA_VERSION,
        "tool": "vacuum",
        "version": env!("CARGO_PKG_VERSION"),
        "contract": READ_ONLY_DOCTOR_CONTRACT,
        "read_only": true,
        "ok": ok,
        "summary": summary,
        "checks": checks,
        "recommended_actions": recommended_actions(ok),
        "fixers": [],
    })
}

fn capabilities_report() -> Value {
    json!({
        "schema_version": CAPABILITIES_SCHEMA_VERSION,
        "tool": "vacuum",
        "version": env!("CARGO_PKG_VERSION"),
        "contract": READ_ONLY_DOCTOR_CONTRACT,
        "read_only": true,
        "network": {
            "required": false,
            "used": false
        },
        "side_effects": {
            "scans_roots": false,
            "reads_file_contents": false,
            "writes_witness_ledger": false,
            "creates_witness_directory": false,
            "writes_doctor_artifacts": false,
            "rewrites_operator_manifest": false,
            "rewrites_schema": false,
            "uses_network": false
        },
        "commands": [
            {
                "command": "vacuum doctor health",
                "json": "vacuum doctor health --json",
                "description": "Run read-only static health checks."
            },
            {
                "command": "vacuum --robot-triage",
                "description": "Emit health, capabilities, output contracts, and recommendations in one JSON payload."
            },
            {
                "command": "vacuum capabilities --json",
                "description": "Describe supported command surfaces and side-effect policy."
            },
            {
                "command": "vacuum robot-docs guide",
                "description": "Print the agent-oriented command guide."
            },
            {
                "command": "vacuum doctor capabilities --json",
                "description": "Describe the doctor command surface and mutation policy."
            },
            {
                "command": "vacuum doctor robot-docs",
                "description": "Print agent-oriented usage notes."
            },
            {
                "command": "vacuum doctor --robot-triage",
                "description": "Emit health and capabilities in one robot-readable report."
            }
        ],
        "detectors": [
            {
                "name": "operator_manifest",
                "mode": "compiled_static_json",
                "mutates": false
            },
            {
                "name": "record_schema",
                "mode": "compiled_static_json",
                "mutates": false
            },
            {
                "name": "witness_path_resolution",
                "mode": "environment_resolution_only",
                "mutates": false
            },
            {
                "name": "scan_stdout_contract",
                "mode": "static_contract",
                "mutates": false
            }
        ],
        "output_contract": {
            "scan_stdout": "JSONL vacuum.v0 records or one refusal envelope",
            "doctor_stdout": "human text or JSON doctor reports",
            "doctor_stderr": "unused on successful doctor commands",
            "exit_codes": {
                "0": "SCAN_COMPLETE or successful read-only discovery command",
                "2": "REFUSAL or CLI usage error"
            }
        },
        "agent_surfaces": {
            "machine_discovery": "vacuum capabilities --json",
            "robot_triage": "vacuum --robot-triage",
            "agent_guide": "vacuum robot-docs guide",
            "scan_jsonl": "vacuum --json <ROOT>..."
        },
        "composition": {
            "family": {
                "name": "cmdrvl-spine",
                "siblings": [
                    {"tool": "vacuum", "capabilities": "vacuum capabilities --json"},
                    {"tool": "hashbytes", "capabilities": "hashbytes capabilities --json"},
                    {"tool": "fingerprint", "capabilities": "fingerprint capabilities --json"},
                    {"tool": "lock", "capabilities": "lock capabilities --json"},
                    {"tool": "pack", "capabilities": "pack capabilities --json"}
                ]
            },
            "role": "start of the evidence stream; enumerate filesystem artifacts without reading file contents",
            "position": 1,
            "accepts": ["directory roots", "file roots"],
            "produces": ["vacuum.v0 JSONL"],
            "canonical_chain": [
                "vacuum --json <ROOT>... | hashbytes | fingerprint --fp <ID> | lock --dataset-id <DATASET> > dataset.lock.json",
                "pack seal dataset.lock.json --output evidence/<DATASET>/"
            ],
            "agent_rules": [
                "Start with vacuum when the input is a directory or file tree.",
                "Pipe vacuum stdout directly to hashbytes; do not parse human text.",
                "Use vacuum capabilities --json when discovering the stream contract programmatically."
            ]
        },
        "fix_mode": {
            "status": "not_available",
            "command": "vacuum doctor --fix",
            "reason": "No vacuum fixer has detector, backup, inverse, and fixture coverage yet.",
            "safe_alternatives": [
                "vacuum doctor --robot-triage",
                "vacuum doctor capabilities --json",
                "vacuum capabilities --json"
            ]
        },
        "fixers": []
    })
}

fn triage_report() -> Value {
    let health = health_report();
    let capabilities = capabilities_report();
    let ok = health.get("ok").cloned().unwrap_or(Value::Bool(false));
    let recommended_actions = health
        .get("recommended_actions")
        .cloned()
        .unwrap_or_else(|| json!([]));
    json!({
        "schema_version": TRIAGE_SCHEMA_VERSION,
        "tool": "vacuum",
        "version": env!("CARGO_PKG_VERSION"),
        "contract": READ_ONLY_DOCTOR_CONTRACT,
        "ok": ok,
        "health": health,
        "capabilities": capabilities,
        "recommended_actions": recommended_actions,
    })
}

fn doctor_v1_report(mode: &str) -> Value {
    let legacy = match mode {
        "health" => health_report(),
        "capabilities" => capabilities_report(),
        "triage" => triage_report(),
        _ => unreachable!("doctor mode is fixed by the caller"),
    };
    let checks = match mode {
        "health" => legacy.get("checks").cloned().unwrap_or_else(|| json!([])),
        "capabilities" => health_report()
            .get("checks")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "triage" => legacy
            .get("health")
            .and_then(|health| health.get("checks"))
            .cloned()
            .unwrap_or_else(|| json!([])),
        _ => unreachable!("doctor mode is fixed by the caller"),
    };
    let findings = checks
        .as_array()
        .into_iter()
        .flatten()
        .map(|check| {
            let status = check
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("error");
            let severity = match status {
                "ok" => "info",
                "warn" => "warning",
                _ => "error",
            };
            let mut finding = json!({
                "id": format!(
                    "vacuum.{}",
                    check.get("name").and_then(Value::as_str).unwrap_or("unknown")
                ),
                "severity": severity,
                "message": check
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("doctor check failed"),
            });
            if severity == "error" {
                finding["fix"] = json!("inspect vacuum doctor capabilities --json");
            }
            finding
        })
        .collect::<Vec<_>>();
    let healthy = !findings
        .iter()
        .any(|finding| finding.get("severity").and_then(Value::as_str) == Some("error"));

    let mut report = json!({
        "schema_version": DOCTOR_SCHEMA_VERSION,
        "mode": mode,
        "tool": "vacuum",
        "tool_version": env!("CARGO_PKG_VERSION"),
        "healthy": healthy,
        "findings": findings,
    });
    match mode {
        "capabilities" => report["contract"] = operator_contract(),
        "triage" => {
            report["next_actions"] = if healthy {
                json!(["vacuum capabilities --json", "vacuum robot-docs guide"])
            } else {
                json!(["vacuum doctor --json", "vacuum doctor capabilities --json"])
            };
        }
        "health" => {}
        _ => unreachable!("doctor mode is fixed by the caller"),
    }
    report
}

fn operator_contract() -> Value {
    serde_json::from_str(OPERATOR_MANIFEST).expect("operator.json must be valid JSON")
}

fn contains_command(manifest: &Value, expected: &str) -> bool {
    manifest
        .get("commands")
        .and_then(Value::as_array)
        .is_some_and(|commands| {
            commands
                .iter()
                .any(|command| command.get("name").and_then(Value::as_str) == Some(expected))
        })
}

fn operator_manifest_check() -> Value {
    let parsed = match serde_json::from_str::<Value>(OPERATOR_MANIFEST) {
        Ok(value) => value,
        Err(error) => {
            return check(
                "operator_manifest",
                "error",
                format!("Compiled operator manifest is invalid JSON: {error}"),
                json!({ "source": "operator.json" }),
            );
        }
    };

    let expected_version = env!("CARGO_PKG_VERSION");
    let name_ok = parsed.get("name").and_then(Value::as_str) == Some("vacuum");
    let schema_ok =
        parsed.get("schema_version").and_then(Value::as_str) == Some(OPERATOR_SCHEMA_VERSION);
    let version_ok = parsed.get("tool_version").and_then(Value::as_str) == Some(expected_version);
    let contract_ok =
        parsed.get("contract_version").and_then(Value::as_str) == Some(CONTRACT_VERSION);
    let commands_ok = ["capabilities", "robot-docs", "doctor"]
        .iter()
        .all(|required| contains_command(&parsed, required));

    if name_ok && schema_ok && version_ok && contract_ok && commands_ok {
        check(
            "operator_manifest",
            "ok",
            "Compiled operator manifest matches the current binary.",
            json!({
                "schema_version": parsed.get("schema_version"),
                "tool_version": parsed.get("tool_version"),
                "contract_version": parsed.get("contract_version"),
                "commands": commands_ok
            }),
        )
    } else {
        check(
            "operator_manifest",
            "error",
            "Compiled operator manifest does not match the current binary contract.",
            json!({
                "expected_name": "vacuum",
                "actual_name": parsed.get("name"),
                "expected_schema_version": OPERATOR_SCHEMA_VERSION,
                "actual_schema_version": parsed.get("schema_version"),
                "expected_version": expected_version,
                "actual_version": parsed.get("tool_version"),
                "expected_contract_version": CONTRACT_VERSION,
                "actual_contract_version": parsed.get("contract_version"),
                "commands": commands_ok
            }),
        )
    }
}

fn schema_manifest_check() -> Value {
    let parsed = match serde_json::from_str::<Value>(schema_manifest()) {
        Ok(value) => value,
        Err(error) => {
            return check(
                "record_schema",
                "error",
                format!("Compiled record schema is invalid JSON: {error}"),
                json!({ "source": "schema/vacuum.v0.schema.json" }),
            );
        }
    };

    if parsed.get("title").and_then(Value::as_str) == Some("vacuum.v0") {
        check(
            "record_schema",
            "ok",
            "Compiled record schema advertises vacuum.v0.",
            json!({
                "title": parsed.get("title"),
                "schema": parsed.get("$schema")
            }),
        )
    } else {
        check(
            "record_schema",
            "error",
            "Compiled record schema title is not vacuum.v0.",
            json!({
                "expected_title": "vacuum.v0",
                "actual_title": parsed.get("title")
            }),
        )
    }
}

fn witness_path_check() -> Value {
    let path = resolve_ledger_path();
    let parent = path.parent();
    let path_status = match fs::metadata(&path) {
        Ok(metadata) if metadata.is_file() => Some(("ok", "Witness ledger path is a file.")),
        Ok(metadata) if metadata.is_dir() => Some((
            "error",
            "Witness ledger path is a directory, not an appendable ledger file.",
        )),
        Ok(_) => Some(("error", "Witness ledger path is not a regular file.")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => Some(("error", "Witness ledger path metadata could not be read.")),
    };
    let (status, message) = path_status.unwrap_or((
        "ok",
        "Witness ledger path is available for future append operations.",
    ));

    check(
        "witness_path_resolution",
        status,
        message,
        json!({
            "path": path.display().to_string(),
            "parent": parent.map(|value| value.display().to_string()),
            "parent_exists": parent.is_some_and(|value| value.exists()),
            "write_attempted": false
        }),
    )
}

fn scan_stdout_contract_check() -> Value {
    check(
        "scan_stdout_contract",
        "ok",
        "Doctor commands are outside the scan path; normal scans still emit vacuum.v0 JSONL.",
        json!({
            "scan_stdout": "vacuum.v0 JSONL or REFUSAL envelope",
            "doctor_stdout": "doctor report",
            "witness_append": false
        }),
    )
}

fn check(name: &str, status: &str, message: impl Into<String>, details: Value) -> Value {
    json!({
        "name": name,
        "status": status,
        "message": message.into(),
        "details": details
    })
}

fn summary_from_checks(checks: &[Value]) -> Value {
    let mut ok = 0;
    let mut warn = 0;
    let mut error = 0;

    for check in checks {
        match check.get("status").and_then(Value::as_str) {
            Some("ok") => ok += 1,
            Some("warn") => warn += 1,
            Some("error") => error += 1,
            _ => error += 1,
        }
    }

    json!({
        "ok": ok,
        "warn": warn,
        "error": error,
        "total": checks.len()
    })
}

fn recommended_actions(ok: bool) -> Vec<&'static str> {
    if ok {
        vec![]
    } else {
        vec!["Inspect the failing compiled manifest or schema check before releasing vacuum."]
    }
}

fn print_health_human(report: &Value) {
    let healthy = report
        .get("healthy")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    println!(
        "vacuum doctor {}",
        if healthy { "healthy" } else { "unhealthy" }
    );

    if let Some(findings) = report.get("findings").and_then(Value::as_array) {
        for finding in findings {
            let severity = finding
                .get("severity")
                .and_then(Value::as_str)
                .unwrap_or("error")
                .to_ascii_uppercase();
            let id = finding
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("<unknown>");
            let message = finding
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("<no message>");
            println!("[{severity}] {id}: {message}");
        }
    }
}

fn print_capabilities_human(report: &Value) {
    println!("vacuum doctor capabilities");
    println!("contract: vacuum capabilities --json");
    println!("healthy: {}", report["healthy"]);
}

fn print_robot_docs() {
    println!("# vacuum robot docs");
    println!();
    println!("`vacuum` scans directories and emits deterministic `vacuum.v0` JSONL on stdout.");
    println!(
        "Use `vacuum --json <ROOT>...` when you want to be explicit; the flag is accepted as a no-op because scan stdout is already machine-readable."
    );
    println!();
    println!("Read-only discovery commands:");
    println!("- `vacuum doctor triage --json` for one canonical cmdrvl.doctor.v1 triage report.");
    println!("- `vacuum --robot-triage` is a deprecated alias for doctor triage.");
    println!("- `vacuum capabilities --json` for command surfaces and side-effect policy.");
    println!("- `vacuum robot-docs guide` for this guide.");
    println!();
    println!("Composition:");
    println!("- `vacuum` starts the CMD+RVL evidence stream and emits `vacuum.v0` JSONL.");
    println!(
        "- Canonical chain: `vacuum --json <ROOT>... | hashbytes | fingerprint --fp <ID> | lock --dataset-id <DATASET> > dataset.lock.json`."
    );
    println!(
        "- Seal the resulting lockfile with `pack seal dataset.lock.json --output evidence/<DATASET>/`."
    );
    println!();
    println!("`vacuum doctor` remains a read-only diagnostic surface for operators.");
    println!(
        "It does not scan roots, read dataset file contents, append witness records, create witness directories, write doctor artifacts, rewrite metadata, or use the network."
    );
    println!();
    println!("Doctor commands:");
    println!("- `vacuum doctor health` for human health output.");
    println!("- `vacuum doctor health --json` for machine-readable health.");
    println!("- `vacuum doctor capabilities --json` for command and side-effect policy.");
    println!("- `vacuum doctor --robot-triage` is a deprecated alias for doctor triage.");
    println!("- An unhealthy doctor report exits 1; CLI errors and refusals exit 2.");
    println!();
    println!(
        "Refusals are JSON envelopes on stdout and include `refusal.next_command` when vacuum can name an actionable next command."
    );
    println!(
        "No fix mode is available. `vacuum doctor --fix` exits 2 and prints safe alternatives to stderr."
    );
}

fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
    );
}

fn print_operator_json() {
    print!("{OPERATOR_MANIFEST}");
    if !OPERATOR_MANIFEST.ends_with('\n') {
        println!();
    }
}

fn exit_for_report(report: &Value) -> u8 {
    if report
        .get("healthy")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        exit::SCAN_COMPLETE
    } else {
        1
    }
}
