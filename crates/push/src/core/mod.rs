use std::{path::PathBuf, str::FromStr};

use eyre::{eyre, Result};
use shadow_common::{forge::ensure_forge_installed, ShadowContractGroupInfo};
use tracing::{error, info};

use crate::{ipfs::pin_shadow_contract_group, PushArgs};

/// The `push` subcommand. Compiles and uploads/pins a shadow contract group to IPFS.
pub async fn push(args: PushArgs) -> Result<()> {
    // ensure forge is installed on the system
    let _ = ensure_forge_installed()?;

    // ensure args are valid
    args.validate().map_err(|e| eyre!("Invalid arguments: {}", e))?;

    // root dir must be a shadow contract group
    let root_dir = PathBuf::from_str(&args.root)?;
    let mut group_info = ShadowContractGroupInfo::from_path(&root_dir)
        .map_err(|e| {
            error!("This is not part of a shadow contract group. You will need to manually add the contract to a group if you wish to pin it to IPFS.");
            eyre!("Failed to load shadow contract group: {}", e)
        })?;

    // validate that the group is ready for pinning
    info!("validating shadow contract group at {}", root_dir.display());
    group_info.validate().map_err(|e| eyre!("Failed to validate shadow contract group: {}", e))?;

    // prepare the group for pinning. this will compile all contracts and build the final
    // IPFS folder structure
    let contract_group_artifact_path = group_info
        .prepare()
        .map_err(|e| eyre!("Failed to prepare shadow contract group: {}", e))?;

    // pin the created folder to IPFS
    info!("pinning shadow contract group to IPFS");
    let pin_result = pin_shadow_contract_group(
        &contract_group_artifact_path,
        &args.pinata_api_key.expect("pinata_api_key should exist"),
        &args.pinata_secret_api_key.expect("pinata_secret_api_key should exist"),
        &args.ipfs_gateway_url,
    )
    .await
    .map_err(|e| eyre!("Failed to pin shadow contract group to IPFS: {}", e))?;
    info!("pinned shadow contract group to IPFS at {}", pin_result.ipfs_url);

    // TODO @jon-becker: must call out to logs.xyz/pin/{} to pin internally

    Ok(())
}
