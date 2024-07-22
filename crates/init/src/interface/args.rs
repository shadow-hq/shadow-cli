use clap::Parser;

/// Arguments for the `init` subcommand
#[derive(Debug, Clone, Parser)]
#[clap(about = "Initialize a new shadow contract group which may be pinned to IPFS")]
pub struct InitArgs {
    /// The path to the directory in which to initialize the shadow contract group.
    #[clap(short, long, default_value = ".", required = false)]
    pub root: String,
}
