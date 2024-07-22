use std::{path::PathBuf, str::FromStr};

use eyre::{eyre, OptionExt, Result};
use shadow_common::ShadowContractGroupInfo;
use tracing::{debug, error, info};

use crate::PushArgs;

/// The `push` subcommand. Compiles and uploads/pins a shadow contract group to IPFS.
pub async fn push(args: PushArgs) -> Result<()> {
    let root_dir = PathBuf::from_str(&args.root)?;

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
    group_info.prepare()?;

    Ok(())
}
