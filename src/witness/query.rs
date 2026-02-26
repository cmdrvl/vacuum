use std::{fs, io::ErrorKind};

use serde_json::{Value, json};

use crate::{
    cli::args::{Command, WitnessAction},
    witness::ledger::resolve_ledger_path,
};

const NO_MATCH_EXIT: u8 = 1;

pub fn dispatch(command: &Command) -> u8 {
    match command {
        Command::Witness { action } => dispatch_witness(action),
    }
}

fn dispatch_witness(action: &WitnessAction) -> u8 {
    let entries = match read_entries() {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("vacuum: witness read failed: {error}");
            return crate::cli::exit::REFUSAL;
        }
    };

    match action {
        WitnessAction::Query {
            tool,
            since,
            until,
            outcome,
            input_hash,
            limit,
            json,
        } => run_query(
            &entries,
            QueryFilter {
                tool: tool.clone(),
                since: since.clone(),
                until: until.clone(),
                outcome: outcome.clone(),
                input_hash: input_hash.clone(),
            },
            *limit,
            *json,
        ),
        WitnessAction::Last { json } => run_last(&entries, *json),
        WitnessAction::Count {
            tool,
            since,
            until,
            outcome,
            input_hash,
            json,
        } => run_count(
            &entries,
            QueryFilter {
                tool: tool.clone(),
                since: since.clone(),
                until: until.clone(),
                outcome: outcome.clone(),
                input_hash: input_hash.clone(),
            },
            *json,
        ),
    }
}

fn run_query(
    entries: &[LedgerEntry],
    filter: QueryFilter,
    limit: Option<usize>,
    json_mode: bool,
) -> u8 {
    let mut matches = entries
        .iter()
        .filter(|entry| entry.matches(&filter))
        .map(|entry| entry.value.clone())
        .collect::<Vec<_>>();

    if let Some(limit) = limit {
        matches.truncate(limit);
    }

    if json_mode {
        println!(
            "{}",
            serde_json::to_string(&matches).unwrap_or_else(|_| "[]".to_string())
        );
    } else {
        for value in &matches {
            println!(
                "{} {} {}",
                value.get("ts").and_then(Value::as_str).unwrap_or("<no-ts>"),
                value
                    .get("outcome")
                    .and_then(Value::as_str)
                    .unwrap_or("<no-outcome>"),
                value
                    .get("tool")
                    .and_then(Value::as_str)
                    .unwrap_or("<no-tool>")
            );
        }
    }

    if matches.is_empty() {
        NO_MATCH_EXIT
    } else {
        crate::cli::exit::SCAN_COMPLETE
    }
}

fn run_last(entries: &[LedgerEntry], json_mode: bool) -> u8 {
    let Some(last) = entries.last() else {
        if json_mode {
            println!("{{}}");
        }
        return NO_MATCH_EXIT;
    };

    if json_mode {
        println!(
            "{}",
            serde_json::to_string(&last.value).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!(
            "{} {} {}",
            last.value
                .get("ts")
                .and_then(Value::as_str)
                .unwrap_or("<no-ts>"),
            last.value
                .get("outcome")
                .and_then(Value::as_str)
                .unwrap_or("<no-outcome>"),
            last.value
                .get("tool")
                .and_then(Value::as_str)
                .unwrap_or("<no-tool>")
        );
    }

    crate::cli::exit::SCAN_COMPLETE
}

fn run_count(entries: &[LedgerEntry], filter: QueryFilter, json_mode: bool) -> u8 {
    let count = entries
        .iter()
        .filter(|entry| entry.matches(&filter))
        .count();

    if json_mode {
        println!("{}", json!({ "count": count }));
    } else {
        println!("{count}");
    }

    if count == 0 {
        NO_MATCH_EXIT
    } else {
        crate::cli::exit::SCAN_COMPLETE
    }
}

fn read_entries() -> Result<Vec<LedgerEntry>, std::io::Error> {
    let path = resolve_ledger_path();
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };

    Ok(contents
        .lines()
        .filter_map(|line| {
            let value = serde_json::from_str::<Value>(line).ok()?;
            Some(LedgerEntry {
                raw: line.to_string(),
                value,
            })
        })
        .collect())
}

struct QueryFilter {
    tool: Option<String>,
    since: Option<String>,
    until: Option<String>,
    outcome: Option<String>,
    input_hash: Option<String>,
}

struct LedgerEntry {
    raw: String,
    value: Value,
}

impl LedgerEntry {
    fn matches(&self, filter: &QueryFilter) -> bool {
        if let Some(tool) = filter.tool.as_deref()
            && self.value.get("tool").and_then(Value::as_str) != Some(tool)
        {
            return false;
        }

        if let Some(outcome) = filter.outcome.as_deref()
            && self.value.get("outcome").and_then(Value::as_str) != Some(outcome)
        {
            return false;
        }

        if let Some(since) = filter.since.as_deref()
            && self.value.get("ts").and_then(Value::as_str).unwrap_or("") < since
        {
            return false;
        }

        if let Some(until) = filter.until.as_deref()
            && self.value.get("ts").and_then(Value::as_str).unwrap_or("") > until
        {
            return false;
        }

        if let Some(input_hash) = filter.input_hash.as_deref()
            && !self.raw.contains(input_hash)
        {
            return false;
        }

        true
    }
}
