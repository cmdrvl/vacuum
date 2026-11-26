use std::{fs, io::ErrorKind};

use serde_json::{Value, json};

use crate::{
    cli::args::WitnessAction,
    witness::ledger::{ensure_ledger_migrated, resolve_ledger_path},
};

pub fn dispatch(action: &WitnessAction) -> u8 {
    let entries = match read_entries() {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("vacuum: witness: {error}");
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
            print_human_row(value);
        }
    }

    // An empty result is a successful read (family ADR): exit 0, `[]` already printed.
    crate::cli::exit::SCAN_COMPLETE
}

fn run_last(entries: &[LedgerEntry], json_mode: bool) -> u8 {
    let Some(last) = entries.last() else {
        // An empty or absent ledger is a successful read: `null` and exit 0 (family ADR).
        if json_mode {
            println!("null");
        }
        return crate::cli::exit::SCAN_COMPLETE;
    };

    if json_mode {
        println!(
            "{}",
            serde_json::to_string(&last.value).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        print_human_row(&last.value);
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

    crate::cli::exit::SCAN_COMPLETE
}

fn read_entries() -> Result<Vec<LedgerEntry>, std::io::Error> {
    ensure_ledger_migrated()?;

    let path = resolve_ledger_path();
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        // A directory, a permission error, or a line that is not valid UTF-8 refuses the read
        // (exit 2) and names the ledger. Vacuum refuses invalid UTF-8 rather than skipping it.
        Err(error) => {
            return Err(std::io::Error::new(
                error.kind(),
                format!("failed to read ledger {}: {error}", path.display()),
            ));
        }
    };

    Ok(contents
        .lines()
        .filter_map(|line| {
            let value = serde_json::from_str::<Value>(line).ok()?;
            Some(LedgerEntry { value })
        })
        .collect())
}

fn print_human_row(value: &Value) {
    println!(
        "ts={} outcome={} tool={}",
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

struct QueryFilter {
    tool: Option<String>,
    since: Option<String>,
    until: Option<String>,
    outcome: Option<String>,
    input_hash: Option<String>,
}

struct LedgerEntry {
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
            && !matches_input_hash(&self.value, input_hash)
        {
            return false;
        }

        true
    }
}

fn matches_input_hash(value: &Value, needle: &str) -> bool {
    value
        .get("inputs")
        .and_then(Value::as_array)
        .is_some_and(|inputs| {
            inputs.iter().any(|input| {
                input
                    .get("hash")
                    .and_then(Value::as_str)
                    .is_some_and(|hash| hash.contains(needle))
            })
        })
        || value
            .get("input_hash")
            .and_then(Value::as_str)
            .is_some_and(|hash| hash.contains(needle))
}
