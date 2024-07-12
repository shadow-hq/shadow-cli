use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[clap(about = "Fetch a contract's source code and metadata from Etherscan.")]
pub struct FetchArgs {}
