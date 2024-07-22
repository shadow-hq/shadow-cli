//! Shadow CLI: An open-source CLI for interacting with the decentralized
//! shadow contract directory, https://logs.xyz .

pub(crate) mod args;

use args::{Arguments, Subcommands};
use clap::Parser;
use eyre::Result;
use shadow_config::Configuration;

#[tokio::main]
async fn main() -> Result<()> {
    // parse command line arguments
    let args = Arguments::parse();

    // init tracing
    let _ = args.logs.init_tracing();

    // load config
    let config = Configuration::load()?;

    match args.sub {
        Subcommands::Config(subargs) => shadow_config::config(subargs)?,
        Subcommands::Compile(subargs) => shadow_compile::compile(subargs).await?,
        Subcommands::Init(subargs) => shadow_init::init(subargs).await?,
        Subcommands::Fetch(mut subargs) => {
            subargs.etherscan_api_key = config.etherscan_api_key;
            shadow_etherscan_fetch::fetch(subargs).await?
        }
        Subcommands::Push(mut subargs) => {
            subargs.pinata_api_key = config.pinata_api_key;
            subargs.pinata_secret_api_key = config.pinata_secret_api_key;
            shadow_push::push(subargs).await?
        }
    };

    Ok(())
}
