use clap::Parser;
use eyre::{OptionExt, Result};

/// Arguments for the `push` subcommand
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

    /// Your preferred IPFS gateway, used when displaying the IPFS URL.
    #[clap(
        long,
        required = false,
        default_value = "https://gateway.pinata.cloud/ipfs/",
        hide_default_value = true
    )]
    pub ipfs_gateway_url: String,
}

impl PushArgs {
    /// Validates the configuration arguments.
    pub fn validate(&self) -> Result<()> {
        let _ = self.pinata_api_key.as_ref().ok_or_eyre(
               "IPFS API key must be set. Use the --pinata-api-key flag or set the IPFS_API_KEY environment variable.")?;
        let _ = self.pinata_secret_api_key.as_ref().ok_or_eyre(
               "IPFS secret API key must be set. Use the --pinata-secret-api-key flag or set the IPFS_SECRET_API_KEY environment variable.")?;

        Ok(())
    }
}
