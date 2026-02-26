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

    if cli.describe {
        output::jsonl::print_operator_stub();
        return cli::exit::SCAN_COMPLETE;
    }

    if cli.schema {
        output::jsonl::print_schema_stub();
        return cli::exit::SCAN_COMPLETE;
    }

    if cli.roots.is_empty() {
        refusal::payload::print_missing_roots();
        return cli::exit::REFUSAL;
    }

    cli::exit::SCAN_COMPLETE
}
