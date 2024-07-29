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
        Subcommands::Init(subargs) => shadow_init::init(subargs).await?,
        Subcommands::Compile(mut subargs) => {
            if let Some(rpc_url) = config.rpc_url {
                subargs.rpc_url = rpc_url;
            }

            shadow_compile::compile(subargs).await?
        }
        Subcommands::Simulate(mut subargs) => {
            if let Some(rpc_url) = config.rpc_url {
                subargs.rpc_url = rpc_url;
            }

            shadow_simulate::simulate(subargs).await?
        }
        Subcommands::Fetch(mut subargs) => {
            subargs.etherscan_api_key = config.etherscan_api_key;
            if let Some(rpc_url) = config.rpc_url {
                subargs.rpc_url = rpc_url;
            }

            shadow_etherscan_fetch::fetch(subargs).await?
        }
        Subcommands::Clone(mut subargs) => {
            subargs.etherscan_api_key = config.etherscan_api_key;
            if let Some(gateway_url) = config.ipfs_gateway_url {
                subargs.ipfs_gateway_url = gateway_url;
            }
            if let Some(rpc_url) = config.rpc_url {
                subargs.rpc_url = rpc_url;
            }

            shadow_clone::clone(subargs).await?
        }
        Subcommands::Push(mut subargs) => {
            subargs.pinata_api_key = config.pinata_api_key;
            subargs.pinata_secret_api_key = config.pinata_secret_api_key;
            if let Some(gateway_url) = config.ipfs_gateway_url {
                subargs.ipfs_gateway_url = gateway_url;
            }
            if let Some(rpc_url) = config.rpc_url {
                subargs.rpc_url = rpc_url;
            }

            shadow_push::push(subargs).await?
        }
    };

    Ok(())
}
