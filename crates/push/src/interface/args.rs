use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[clap(about = "Compiles and uploads/pins a shadow contract group to IPFS")]
pub struct PushArgs {
    /// The path to the directory in which to initialize the shadow contract group.
    #[clap(short, long, default_value = ".", required = false)]
    pub root: String,
}
