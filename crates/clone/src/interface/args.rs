use std::str::FromStr;

use alloy_chains::{Chain, NamedChain};
use clap::Parser;
use eyre::{eyre, Result};

/// Arguments for the `clone` subcommand
#[derive(Debug, Clone, Parser)]
#[clap(about = "Clones a shadow contract group from IPFS and saves it to the local filesystem")]
pub struct CloneArgs {
    /// The ipfs CID of the contract group to fetch.
    pub ipfs_cid: String,

    /// The API key to use for Etherscan.
    #[clap(short, long, required = false)]
    pub etherscan_api_key: Option<String>,

    /// Chain to fetch the contract from. Defaults to `ethereum`.
    #[clap(short, long, required = false)]
    pub chain: Option<String>,

    /// Chain ID to fetch the contract from. Defaults to `1`.
    #[clap(short = 'i', long, required = false)]
    pub chain_id: Option<u64>,

    /// The path to the directory or contract group in which to save the fetched contract.
    #[clap(short, long, default_value = ".", required = false)]
    pub root: String,

    /// Whether to force overwrite the existing files.
    #[clap(short, long)]
    pub force: bool,

    /// Your preferred IPFS gateway, used when displaying the IPFS URL.
    #[clap(
        long,
        required = false,
        default_value = "https://gateway.pinata.cloud/ipfs/",
        hide_default_value = true
    )]
    pub ipfs_gateway_url: String,

    /// The RPC URL of the chain to simulate the transaction on.
    #[clap(short = 'u', long, default_value = "http://localhost:8545")]
    pub rpc_url: String,
}

impl TryFrom<CloneArgs> for Chain {
    type Error = eyre::Error;

    fn try_from(args: CloneArgs) -> Result<Self> {
        let chain = match (args.chain, args.chain_id) {
            (Some(chain), _) => Chain::from_named(
                NamedChain::from_str(&chain).map_err(|_| eyre!("Invalid chain name: {}", chain))?,
            ),
            (None, Some(chain_id)) => Chain::from_id(chain_id),
            (None, None) => Chain::mainnet(),
        };
        Ok(chain)
    }
}
