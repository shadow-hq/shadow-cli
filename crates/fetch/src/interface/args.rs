use alloy::{
    network::AnyNetwork,
    providers::{Provider, ProviderBuilder},
    transports::http::reqwest::Url,
};
use alloy_chains::Chain;
use clap::Parser;

/// Arguments for the `fetch` subcommand
#[derive(Debug, Clone, Parser)]
#[clap(about = "Fetch a contract's source code and metadata from Etherscan or Blockscout.")]
pub struct FetchArgs {
    /// The address of the contract to fetch
    pub address: String,

    /// The API key to use for Etherscan.
    #[clap(short, long, required = false)]
    pub etherscan_api_key: Option<String>,

    /// The path to the directory or contract group in which to save the fetched contract.
    #[clap(short, long, default_value = ".", required = false)]
    pub root: String,

    /// Whether to force overwrite the existing files.
    #[clap(short, long)]
    pub force: bool,

    /// The RPC URL of the chain to simulate the transaction on.
    #[clap(short = 'u', long, default_value = "http://localhost:8545")]
    pub rpc_url: String,

    /// The blockscan URL to use for fetching contract metadata
    #[clap(short, long)]
    pub blockscout_url: Option<String>,

    /// Whether to save the compiled contract to '{root}/shadow.json' for use with shadow-reth.
    #[clap(long)]
    pub reth: bool,
}

impl FetchArgs {
    /// Try to get the chain ID from the RPC URL
    pub async fn try_get_chain(&self) -> eyre::Result<Chain> {
        let provider =
            ProviderBuilder::new().network::<AnyNetwork>().on_http(Url::parse(&self.rpc_url)?);

        let chain_id = provider
            .get_chain_id()
            .await
            .map_err(|e| eyre::eyre!("failed to get chain ID from RPC: {}", e))?;

        Ok(Chain::from_id_unchecked(chain_id))
    }
}
