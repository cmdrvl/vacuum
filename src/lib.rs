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
        let rendered = refusal::payload::render(&refusal);
        println!("{rendered}");
        append_witness_record(
            &cli,
            "REFUSAL",
            cli::exit::REFUSAL,
            hash_bytes(format!("{rendered}\n").as_bytes()),
        );
        return cli::exit::REFUSAL;
    }

    if let Err(refusal) = walk::walker::validate_roots(&cli.roots) {
        let rendered = refusal::payload::render(&refusal);
        println!("{rendered}");
        append_witness_record(
            &cli,
            "REFUSAL",
            cli::exit::REFUSAL,
            hash_bytes(format!("{rendered}\n").as_bytes()),
        );
        return cli::exit::REFUSAL;
    }

    let scanned = walk::walker::scan_roots_with_progress(&cli.roots, !cli.no_follow, cli.progress);
    let filtered = walk::filter::apply_filters(scanned, &cli.include, &cli.exclude);
    let rendered_lines = output::jsonl::serialize_sorted_jsonl(&filtered);
    for line in &rendered_lines {
        println!("{line}");
    }
    let output_bytes = rendered_lines
        .iter()
        .flat_map(|line| {
            line.as_bytes()
                .iter()
                .copied()
                .chain(std::iter::once(b'\n'))
        })
        .collect::<Vec<_>>();
    let output_hash = hash_bytes(&output_bytes);
    append_witness_record(&cli, "SCAN_COMPLETE", cli::exit::SCAN_COMPLETE, output_hash);

    cli::exit::SCAN_COMPLETE
}

fn append_witness_record(cli: &cli::args::Cli, outcome: &str, exit_code: u8, output_hash: String) {
    if cli.no_witness {
        return;
    }

    let mut record = witness::record::WitnessRecord::from_run(
        &cli.roots,
        &cli.include,
        &cli.exclude,
        cli.no_follow,
        outcome,
        exit_code,
        output_hash,
        witness::ledger::read_prev(),
    );
    record.compute_id();
    if let Err(error) = witness::ledger::append(&record) {
        emit_witness_warning(cli.progress, &error);
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

fn hash_bytes(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}
