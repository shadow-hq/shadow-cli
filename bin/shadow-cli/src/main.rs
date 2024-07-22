//! Shadow CLI: An open-source CLI for interacting with the decentralized
//! shadow contract directory, https://logs.xyz .

pub(crate) mod args;

use args::{Arguments, Subcommands};
use clap::Parser;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // parse command line arguments
    let args = Arguments::parse();

    // init tracing
    let _ = args.logs.init_tracing();

    match args.sub {
        Subcommands::Config(subargs) => shadow_config::config(subargs)?,
        Subcommands::Fetch(subargs) => shadow_etherscan_fetch::fetch(subargs).await?,
        Subcommands::Compile(subargs) => shadow_compile::compile(subargs).await?,
        Subcommands::Init(subargs) => shadow_init::init(subargs).await?,
        Subcommands::Push(subargs) => shadow_push::push(subargs).await?,
    };

    Ok(())
}
