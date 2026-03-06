#![forbid(unsafe_code)]

use serde_json::json;

pub mod cli;
pub mod output;
pub mod record;
pub mod refusal;
pub mod walk;
pub mod witness;

pub fn run() -> u8 {
    let cli = match cli::args::parse() {
        Ok(cli) => cli,
        Err(error) => return cli::exit::from_clap_error(error),
    };

    if let Some(command) = cli.command.as_ref() {
        return witness::query::dispatch(command);
    }

    if cli.version {
        println!("vacuum {}", env!("CARGO_PKG_VERSION"));
        return cli::exit::SCAN_COMPLETE;
    }

    if cli.describe {
        output::jsonl::print_operator_manifest();
        return cli::exit::SCAN_COMPLETE;
    }

    if cli.schema {
        output::jsonl::print_schema_manifest();
        return cli::exit::SCAN_COMPLETE;
    }

    if cli.roots.is_empty() {
        let refusal = refusal::payload::empty_roots_refusal();
        refusal::payload::emit(&refusal);
        append_witness_record(cli.no_witness, cli.progress, "REFUSAL", cli::exit::REFUSAL);
        return cli::exit::REFUSAL;
    }

    if let Err(refusal) = walk::walker::validate_roots(&cli.roots) {
        refusal::payload::emit(&refusal);
        append_witness_record(cli.no_witness, cli.progress, "REFUSAL", cli::exit::REFUSAL);
        return cli::exit::REFUSAL;
    }

    let scanned = walk::walker::scan_roots_with_progress(&cli.roots, !cli.no_follow, cli.progress);
    let filtered = walk::filter::apply_filters(scanned, &cli.include, &cli.exclude);
    output::jsonl::emit_records(&filtered);
    append_witness_record(
        cli.no_witness,
        cli.progress,
        "SCAN_COMPLETE",
        cli::exit::SCAN_COMPLETE,
    );

    cli::exit::SCAN_COMPLETE
}

fn append_witness_record(no_witness: bool, progress_enabled: bool, outcome: &str, exit_code: u8) {
    if no_witness {
        return;
    }

    let record = witness::record::WitnessRecord::new(outcome, exit_code);
    if let Err(error) = witness::ledger::append(&record) {
        emit_witness_warning(progress_enabled, &error);
    }
}

fn emit_witness_warning(progress_enabled: bool, error: &std::io::Error) {
    if progress_enabled {
        let payload = json!({
            "type": "warning",
            "tool": "vacuum",
            "message": format!("Witness append failed: {error}"),
        });
        eprintln!("{payload}");
    } else {
        eprintln!("vacuum: witness append failed: {error}");
    }
}
