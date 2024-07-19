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

    /// The output directory root to save the contract source code and metadata.
    #[clap(short, long, default_value = ".", required = false)]
    pub output: String,

    /// Whether to force overwrite the existing files.
    #[clap(short, long)]
    pub force: bool,
}
