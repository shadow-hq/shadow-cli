//! Shadow CLI: An open-source CLI for interacting with the decentralized
//! shadow contract directory, https://logs.xyz .

pub(crate) mod args;

use args::{Arguments, Subcommands};
use clap::Parser;
use eyre::Result;
use shadow_common::version::*;
use shadow_config::Configuration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // parse command line arguments
    let args = Arguments::parse();

    // init tracing
    let _ = args.logs.init_tracing();

    // spawn a new tokio runtime to get remote version while the main runtime is running
    let current_version = current_version();
    let remote_ver = if current_version.is_nightly() {
        tokio::task::spawn(remote_nightly_version()).await??
    } else {
        tokio::task::spawn(remote_version()).await??
    };

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
            if let Some(etherscan_api_key) = config.etherscan_api_key {
                subargs.etherscan_api_key = Some(etherscan_api_key);
            }
            if let Some(rpc_url) = config.rpc_url {
                subargs.rpc_url = rpc_url;
            }

            shadow_etherscan_fetch::fetch(subargs).await?
        }
        Subcommands::Clone(mut subargs) => {
            if let Some(etherscan_api_key) = config.etherscan_api_key {
                subargs.etherscan_api_key = Some(etherscan_api_key);
            }
            if let Some(gateway_url) = config.ipfs_gateway_url {
                subargs.ipfs_gateway_url = gateway_url;
            }
            if let Some(rpc_url) = config.rpc_url {
                subargs.rpc_url = rpc_url;
            }

            shadow_clone::clone(subargs).await?
        }
        Subcommands::Push(mut subargs) => {
            if let Some(pinata_api_key) = config.pinata_api_key {
                subargs.pinata_api_key = Some(pinata_api_key)
            }
            if let Some(pinata_secret_api_key) = config.pinata_secret_api_key {
                subargs.pinata_secret_api_key = Some(pinata_secret_api_key)
            }
            if let Some(gateway_url) = config.ipfs_gateway_url {
                subargs.ipfs_gateway_url = gateway_url;
            }
            if let Some(rpc_url) = config.rpc_url {
                subargs.rpc_url = rpc_url;
            }

            shadow_push::push(subargs).await?
        }
    };

    // check if the version is up to date
    if current_version.is_nightly() && current_version.ne(&remote_ver) {
        info!("great news! A new nightly build is available!");
        info!("you can update now by running: `shadowup +nightly`");
    } else if remote_ver.gt(&current_version) {
        info!("great news! An update is available!");
        info!("you can update now by running: `shadowup --version {}`", remote_ver);
    }

    Ok(())
}
