use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "vacuum",
    about = "Enumerate artifacts and emit deterministic JSONL manifests",
    long_about = None
)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(value_name = "ROOT")]
    pub roots: Vec<PathBuf>,

    #[arg(long, action = ArgAction::Append, value_name = "GLOB")]
    pub include: Vec<String>,

    #[arg(long, action = ArgAction::Append, value_name = "GLOB")]
    pub exclude: Vec<String>,

    #[arg(long)]
    pub no_follow: bool,

    #[arg(long)]
    pub no_witness: bool,

    #[arg(long)]
    pub describe: bool,

    #[arg(long)]
    pub schema: bool,

    #[arg(long)]
    pub progress: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Witness {
        #[command(subcommand)]
        action: WitnessAction,
    },
}

#[derive(Debug, Subcommand)]
pub enum WitnessAction {
    Query {
        #[arg(long)]
        tool: Option<String>,
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        until: Option<String>,
        #[arg(long)]
        outcome: Option<String>,
        #[arg(long = "input-hash")]
        input_hash: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        json: bool,
    },
    Last {
        #[arg(long)]
        json: bool,
    },
    Count {
        #[arg(long)]
        tool: Option<String>,
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        until: Option<String>,
        #[arg(long)]
        outcome: Option<String>,
        #[arg(long = "input-hash")]
        input_hash: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

pub fn parse() -> Result<Cli, clap::Error> {
    Cli::try_parse()
}
