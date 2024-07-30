use clap::Parser;
use eyre::Result;

/// Arguments for the `sim` subcommand
#[derive(Debug, Clone, Parser)]
#[clap(about = "Simulate a transaction with shadow overrides")]
pub struct SimulateArgs {
    /// The transaction hash to simulate.
    pub transaction_hash: String,

    /// The path to the directory in which to initialize the shadow contract group.
    #[clap(short, long, default_value = ".", required = false)]
    pub root: String,

    /// The RPC URL of the chain to simulate the transaction on.
    #[clap(short = 'u', long, default_value = "http://localhost:8545")]
    pub rpc_url: String,
}

impl SimulateArgs {
    /// Validates the configuration arguments.
    pub fn validate(&self) -> Result<()> {
        Ok(())
    }
}
