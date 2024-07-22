use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[clap(about = "Compiles and uploads/pins a shadow contract group to IPFS")]
pub struct PushArgs {
    /// The path to the directory in which to initialize the shadow contract group.
    #[clap(short, long, default_value = ".", required = false)]
    pub root: String,

    /// Your pinata API key, used to pin the shadow contract group to IPFS.
    #[clap(long, required = false, alias = "ipfs-api-key")]
    pub pinata_api_key: Option<String>,

    /// Your pinata secret API key, used to pin the shadow contract group to IPFS.
    #[clap(long, required = false, alias = "ipfs-secret-api-key")]
    pub pinata_secret_api_key: Option<String>,
}
