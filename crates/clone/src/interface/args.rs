use clap::Parser;

/// Arguments for the `clone` subcommand
#[derive(Debug, Clone, Parser)]
#[clap(about = "Clones a shadow contract group from IPFS and saves it to the local filesystem")]
pub struct CloneArgs {
    /// The ipfs CID of the contract group to fetch.
    pub ipfs_cid: String,

    /// The API key to use for Etherscan.
    #[clap(short, long, required = false)]
    pub etherscan_api_key: Option<String>,

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

    /// The blockscan URL to use for fetching contract metadata
    #[clap(short, long)]
    pub blockscout_url: Option<String>,

    /// Whether to save the compiled contract to '{root}/shadow.json' for use with shadow-reth.
    #[clap(long)]
    pub reth: bool,
}
