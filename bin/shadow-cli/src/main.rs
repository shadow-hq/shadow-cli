//! Shadow CLI: An open-source CLI for interacting with the decentralized
//! shadow contract directory, https://logs.xyz .

pub(crate) mod log_args;
pub(crate) mod subcommands;

use clap::{Parser, Subcommand};
use eyre::Result;
use log_args::LogArgs;
use subcommands::*;

#[derive(Debug, Parser)]
pub struct Arguments {
    #[clap(subcommand)]
    pub sub: Subcommands,

    #[clap(flatten)]
    logs: LogArgs,
}

#[derive(Debug, Subcommand)]
#[clap(
    about = "Shadow CLI: An open-source CLI for interacting with the decentralized shadow contract directory.",
    after_help = "For more information, check out https://logs.xyz"
)]
#[allow(clippy::large_enum_variant)]
pub enum Subcommands {
    #[clap(name = "config", about = "Display or edit your shadow CLI configuration.")]
    Config(ConfigArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    // parse command line arguments
    let args = Arguments::parse();

    // init tracing
    let _ = args.logs.init_tracing();

    match args.sub {
        Subcommands::Config(subargs) => config(subargs)?,
    };

    Ok(())
}
