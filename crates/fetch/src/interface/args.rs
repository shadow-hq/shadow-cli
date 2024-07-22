use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[clap(about = "Fetch a contract's source code and metadata from Etherscan.")]
pub struct FetchArgs {
    /// The address of the contract to fetch
    pub address: String,

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
}
