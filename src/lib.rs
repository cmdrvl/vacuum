#![forbid(unsafe_code)]

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
        return cli::exit::REFUSAL;
    }

    if let Err(refusal) = walk::walker::validate_roots(&cli.roots) {
        refusal::payload::emit(&refusal);
        return cli::exit::REFUSAL;
    }

    let scanned = walk::walker::scan_roots(&cli.roots, !cli.no_follow);
    let filtered = walk::filter::apply_filters(scanned, &cli.include, &cli.exclude);
    output::jsonl::emit_records(&filtered);

    cli::exit::SCAN_COMPLETE
}
