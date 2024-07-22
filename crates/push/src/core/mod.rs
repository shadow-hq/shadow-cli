use std::{path::PathBuf, str::FromStr};

use eyre::{bail, eyre, OptionExt, Result};
use pinata_sdk::{PinByFile, PinataApi};
use shadow_common::ShadowContractGroupInfo;
use tracing::{debug, error, info};

use crate::PushArgs;

/// The `push` subcommand. Compiles and uploads/pins a shadow contract group to IPFS.
pub async fn push(args: PushArgs) -> Result<()> {
    let root_dir = PathBuf::from_str(&args.root)?;
    let ipfs_api_key = args.pinata_api_key.ok_or_eyre(
           "IPFS API key must be set. Use the --pinata-api-key flag or set the IPFS_API_KEY environment variable.")?;
    let ipfs_secret_api_key = args.pinata_secret_api_key.ok_or_eyre(
           "IPFS secret API key must be set. Use the --pinata-secret-api-key flag or set the IPFS_SECRET_API_KEY environment variable.")?;

    info!("validating shadow contract group at {}", root_dir.display());

    // root dir must be a shadow contract group
    let mut group_info = match ShadowContractGroupInfo::from_path(&root_dir) {
        Ok(group_info) => group_info,
        Err(_) => {
            error!("This is not part of a shadow contract group. You will need to manually add the contract to a group if you wish to pin it to IPFS.");
            return Err(eyre!("not part of a shadow contract group"));
        }
    };

    // group must have a display name
    if &group_info.display_name == "Unnamed Contract Group" {
        error!("This is an unnamed contract group. You must name the group in {}/info.json before pushing.", root_dir.display());
        return Err(eyre!("unnamed contract group"));
    }

    // creator must exist
    if group_info.creator.is_none() {
        error!("This contract group has no creator. You must add a creator address in {}/info.json before pushing.", root_dir.display());
        return Err(eyre!("no creator"));
    }

    // update the group_info
    group_info.update_contracts()?;

    // prepare the group for pinning
    let contract_group_artifact_path = group_info.prepare()?;
    let contract_group_artifact_path =
        format!("{}/", contract_group_artifact_path.to_string_lossy().to_string());

    info!("pinning shadow contract group to IPFS");
    let api = PinataApi::new(&ipfs_api_key, &ipfs_secret_api_key)
        .map_err(|e| eyre!("Failed to create Pinata API client: {}", e))?;

    println!("contract_group_artifact_path: {:?}", contract_group_artifact_path);
    let result = api
        .pin_file(PinByFile::new(contract_group_artifact_path))
        .await
        .map_err(|e| eyre!("Failed to pin file: {}", e))?;

    println!("result: {:?}", result);

    Ok(())
}
