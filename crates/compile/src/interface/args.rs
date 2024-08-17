use clap::Parser;

/// Arguments for the `compile` subcommand
#[derive(Debug, Clone, Parser)]
#[clap(about = "Compile a shadowed contract with the original contract settings")]
pub struct CompileArgs {
    /// The project's root path
    #[clap(short, long, default_value = ".", hide_default_value = true)]
    pub root: String,

    /// The RPC URL of the chain to simulate the transaction on.
    #[clap(short = 'u', long, default_value = "http://localhost:8545")]
    pub rpc_url: String,

    /// Whether to save the compiled contract to '{root}/shadow.json' for use with shadow-reth.
    #[clap(long)]
    pub reth: bool,
}
