use std::{path::PathBuf, str::FromStr};

use crate::FetchArgs;
use eyre::{eyre, Result};
use foundry_block_explorers::Client as EtherscanClient;
use shadow_common::{
    blockscout::Client as BlockscoutClient, compiler, forge::ensure_forge_installed,
    ShadowContractGroupInfo, ShadowContractInfo, ShadowContractSettings, ShadowContractSource,
};
use tracing::{error, info, trace, warn};

/// The `fetch` subcommand. Fetches a contract's source code and metadata from Etherscan or
/// Blockscout, and saves it locally.
pub async fn fetch(args: FetchArgs) -> Result<()> {
    // ensure forge is installed on the system
    ensure_forge_installed()?;

    let chain = args.try_get_chain().await?;
    trace!("using chain {}", chain);

    // check if this is part of a shadow contract group
    let mut output_dir = PathBuf::from_str(&args.root)?;
    let mut group_info = match ShadowContractGroupInfo::from_path(&output_dir) {
        Ok(group_info) => {
            // we need to update the output path under `output_dir/chain_id/contract_address`
            output_dir.push(chain.id().to_string());
            output_dir.push(args.address.to_lowercase());
            Some(group_info)
        }
        Err(_) => {
            warn!("This is not part of a shadow contract group. You will need to manually add the contract to a group if you wish to pin it to IPFS.");
            None
        }
    };

    // output_dir must be empty, unless --force is set
    if output_dir.exists() && output_dir.read_dir()?.next().is_some() {
        if args.force {
            std::fs::remove_dir_all(&output_dir)?;
        } else {
            error!("Output directory already exists. Use --force to overwrite.");
            return Err(eyre!("output directory already exists"));
        }
    }

    // fetch contract metadata and creation data
    let address = args.address.parse().map_err(|_| eyre!("Invalid address: {}", args.address))?;
    let (metadata, creation_data) = if let Some(blockscout_url) = args.blockscout_url {
        let client = BlockscoutClient::new(&blockscout_url);
        let metadata = client.contract_source_code(address).await?;
        let creation_data = client.contract_creation_data(address).await?;

        (metadata, creation_data)
    } else {
        let client = EtherscanClient::new(chain, args.etherscan_api_key.unwrap_or_default())?;
        let metadata = client.contract_source_code(address).await?;
        let creation_data = client.contract_creation_data(address).await?;

        (metadata, creation_data)
    };

    let info = ShadowContractInfo::new(&chain, &metadata, &creation_data);
    let source = ShadowContractSource::new(&metadata)?;
    let settings = ShadowContractSettings::new(&metadata);

    info!("successfully fetched contract information from etherscan");
    info!("writing contract to {}", output_dir.display());

    // initialize foundry project structure
    init_via_forge(&output_dir)
        .map_err(|e| eyre!("failed to initialize foundry project: {}", e))?;

    // create directories
    std::fs::create_dir_all(output_dir.clone())?;
    let info_path = output_dir.join("info.json");
    let source_path = output_dir.join("source.json");
    let original_source_path = output_dir.join("original.json");
    let settings_path = output_dir.join("settings.json");
    let src_dir = output_dir.join("src");
    let test_dir = output_dir.join("test");
    let script_dir = output_dir.join("script");

    // serialize to json
    let info_json = serde_json::to_string_pretty(&info)?;
    let source_json = serde_json::to_string_pretty(&source)?;
    let settings_json = serde_json::to_string_pretty(&settings)?;

    // write files
    std::fs::write(info_path, info_json)?;
    std::fs::write(source_path, &source_json)?;
    std::fs::write(settings_path, settings_json)?;
    std::fs::write(original_source_path, source_json)?;
    std::fs::remove_dir_all(src_dir.clone())?;
    std::fs::remove_dir_all(test_dir.clone())?;
    std::fs::remove_dir_all(script_dir.clone())?;
    std::fs::create_dir_all(src_dir)?;
    std::fs::create_dir_all(test_dir)?;
    std::fs::create_dir_all(script_dir)?;

    // rebuild source
    source.write_source_to(&output_dir)?;
    settings.generate_config(&output_dir)?;

    // update shadow contract group info
    if let Some(group_info) = group_info.as_mut() {
        group_info.update_contracts()?;
    }

    compiler::compile(&args.rpc_url, &output_dir, &settings, &info).await?;

    Ok(())
}

/// Initializes a new foundry project in the specified directory using the `forge` CLI.
fn init_via_forge(output_dir: &PathBuf) -> Result<()> {
    let status = std::process::Command::new("forge")
        .arg("init")
        .arg(output_dir)
        .arg("--no-git")
        .arg("--no-commit")
        .arg("--quiet")
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .status()
        .expect("`forge init` failed.");
    if !status.success() {
        error!("`forge init` failed.");
        std::process::exit(1);
    }

    Ok(())
}
