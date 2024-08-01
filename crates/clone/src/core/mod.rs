use std::{path::PathBuf, str::FromStr};

use crate::{ipfs::read_from_ipfs, CloneArgs};
use eyre::Result;
use shadow_common::{forge::ensure_forge_installed, ShadowContractGroupInfo, ShadowContractSource};
use shadow_etherscan_fetch::FetchArgs;

use tracing::{debug, info};

/// The `clone` subcommand. Clones a shadow contract group from IPFS and saves it to the local
/// filesystem
pub async fn clone(args: CloneArgs) -> Result<()> {
    // ensure forge is installed on the system
    ensure_forge_installed()?;

    // get the contract group's metadata from IPFS
    info!("fetching contract group metadata from IPFS...");
    let metadata: ShadowContractGroupInfo =
        read_from_ipfs(&format!("{}/info.json", args.ipfs_cid), &args.ipfs_gateway_url).await?;

    let parent = PathBuf::from_str(&args.root)?;
    let root = metadata.write_folder_structure(parent)?;

    // for each contract in the group, call `shadow fetch` to build a working foundry environment
    // for each contract. we will apply source diffs later.
    for contract in metadata.contracts {
        info!("fetching contract: {}", contract.address);
        shadow_etherscan_fetch::fetch(FetchArgs {
            address: contract.address.to_string(),
            etherscan_api_key: args.etherscan_api_key.clone(),
            root: root.to_string_lossy().to_string(),
            force: args.force,
            rpc_url: args.rpc_url.clone(),
            blockscout_url: args.blockscout_url.clone(),
        })
        .await?;

        // apply source diffs
        debug!("applying source diffs for contract: {}", contract.address);
        let shadow_source: ShadowContractSource = read_from_ipfs(
            &format!(
                "{}/{}/{}/source.json",
                args.ipfs_cid,
                contract.chain_id,
                contract.address.to_string().to_lowercase()
            ),
            &args.ipfs_gateway_url,
        )
        .await?;

        let src_path = root
            .join(contract.chain_id.to_string())
            .join(contract.address.to_string().to_lowercase())
            .join("src");
        shadow_source.write_source_to(&src_path)?;

        info!("successfully cloned contract: {}", contract.address);
    }

    info!("successfully cloned contract group: {}", args.ipfs_cid);

    Ok(())
}
