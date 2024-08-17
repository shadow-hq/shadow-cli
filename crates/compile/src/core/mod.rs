use std::{collections::HashMap, path::PathBuf, str::FromStr};

use crate::CompileArgs;
use eyre::{eyre, Result};
use shadow_common::{
    compiler, forge::ensure_forge_installed, ShadowContractInfo, ShadowContractSettings,
};
use tracing::info;

/// The `compile` subcommand. Compiles a shadowed contract with the original contract settings.
pub async fn compile(args: CompileArgs) -> Result<()> {
    // ensure forge is installed on the system
    ensure_forge_installed()?;

    let root_dir = PathBuf::from_str(&args.root)?;
    let settings_path = root_dir.join("settings.json");
    let info_path = root_dir.join("info.json");

    // ensure settings and info.json exist, load them
    let settings: ShadowContractSettings = serde_json::from_slice(&std::fs::read(settings_path)
        .map_err(|e| eyre!("expected settings.json in root directory. you may need to run `shadow fetch` first: {}", e))?
    )?;
    let info: ShadowContractInfo = serde_json::from_slice(&std::fs::read(info_path)
        .map_err(|e| eyre!("expected info.json in root directory. you may need to run `shadow fetch` first: {}", e))?
    )?;

    // compile the contract with the original settings
    let start_time = std::time::Instant::now();
    info!("compiling contract {} with {}...", info.name, settings.compiler_version);
    let compiler_output = compiler::compile(&args.rpc_url, &root_dir, &settings, &info).await?;
    info!("compiled successfully in {}ms", start_time.elapsed().as_millis());

    if args.reth {
        // check for `shadow.json` in the root directory and load it if it exists.
        let mut reth_config = match std::fs::read_to_string("shadow.json") {
            Ok(contents) => {
                serde_json::from_str::<HashMap<String, String>>(&contents).unwrap_or_default()
            }
            Err(_) => HashMap::new(),
        };

        // update the reth config with the new contract
        reth_config.insert(
            format!("{}", info.address),
            format!("0x{}", hex::encode(&compiler_output.bytecode)),
        );

        // write the updated reth config to `shadow.json`
        let reth_config_json = serde_json::to_string_pretty(&reth_config)?;
        std::fs::write("shadow.json", reth_config_json)?;
        info!("updated shadow-reth configuration");
    }

    Ok(())
}
