use std::{path::PathBuf, str::FromStr};

use eyre::{eyre, OptionExt, Result};
use shadow_common::ShadowContractGroupInfo;
use tracing::info;

use crate::InitArgs;

/// The `init` subcommand. Initialize a new shadow contract group which may be pinned to IPFS.
pub async fn init(args: InitArgs) -> Result<()> {
    let output_dir = PathBuf::from_str(&args.root)?;

    let path = ShadowContractGroupInfo::default().write_folder_structure(output_dir)?;

    info!("initialized new shadow contract group at {}", path.display());

    Ok(())
}
