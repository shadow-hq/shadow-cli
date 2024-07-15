use std::{env::temp_dir, os::unix::process::CommandExt, path::PathBuf, str::FromStr};

use crate::FetchArgs;
use alloy_chains::{Chain, NamedChain};
use eyre::{eyre, OptionExt, Result};
use foundry_block_explorers::Client;
use shadow_common::{compiler, ShadowContractInfo, ShadowContractSettings, ShadowContractSource};
use tracing::{error, info, trace, warn};
use which::Path;

/// The `fetch` subcommand. Fetches a contract's source code and metadata from Etherscan, and
/// saves it locally.
/// TODO: @jon-becker clean this up w/ helpers rather than a single function
/// TODO: @jon-becker --force flag
pub async fn fetch(args: FetchArgs) -> Result<()> {
    // ensure `forge` is installed with `which forge`
    let _ = match which::which("forge") {
        Ok(_) => (),
        Err(_) => {
            const YELLOW_ANSI_CODE: &str = "\u{001b}[33m";
            const LIGHT_GRAY_ANSI_CODE: &str = "\u{001b}[90m";
            const RESET_ANSI_CODE: &str = "\u{001b}[0m";
            print!(
                "{LIGHT_GRAY_ANSI_CODE}{}  {YELLOW_ANSI_CODE}WARN{RESET_ANSI_CODE} `forge` is not installed. would you like to install it now? [Y/n] ",
                // include microsecond precision
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
            );
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "" {
                info!("Installing foundryup via `curl -L https://foundry.paradigm.xyz | bash`");

                // silently install foundryup via bash
                let status = std::process::Command::new("bash")
                    .arg("-c")
                    .arg("curl -L https://foundry.paradigm.xyz | bash")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .expect("Failed to install `foundryup`.");

                if !status.success() {
                    error!("Failed to install `foundryup`.");
                    std::process::exit(1);
                }

                // silently run foundryup
                info!("Installing forge via `foundryup`");
                let status = std::process::Command::new("foundryup")
                    .stderr(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .status()
                    .expect("Failed to install `forge`.");

                if !status.success() {
                    error!("Failed to install `forge`.");
                    std::process::exit(1);
                }

                info!("Successfully installed `forge`.");
            } else {
                error!("`forge` is required by this command. Please install it and try again.");
                std::process::exit(1);
            }
        }
    };

    let chain = match (args.chain, args.chain_id) {
        (Some(chain), _) => Chain::from_named(
            NamedChain::from_str(&chain).map_err(|_| eyre!("Invalid chain name: {}", chain))?,
        ),
        (None, Some(chain_id)) => Chain::from_id(chain_id),
        (None, None) => Chain::mainnet(),
    };
    trace!("using chain: {} ({})", chain, chain.id());

    // fetch contract metadata and creation data
    let client = Client::new(chain, args.etherscan_api_key.unwrap_or_default())?;
    let address = args.address.parse().map_err(|_| eyre!("Invalid address: {}", args.address))?;
    let metadata = client.contract_source_code(address).await?;
    let creation_data = client.contract_creation_data(address).await?;

    let info = ShadowContractInfo::new(&chain, &metadata, &creation_data);
    let source = ShadowContractSource::new(&metadata);
    let settings = ShadowContractSettings::new(&metadata);

    info!("successfully fetched contract information from etherscan");

    // run forge init --no-git --no-commit
    let status = std::process::Command::new("forge")
        .arg("init")
        .arg(&args.output)
        .arg("--no-git")
        .arg("--no-commit")
        .arg("--quiet")
        .stderr(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .status()
        .expect("`forge init` failed.");
    if !status.success() {
        error!("`forge init` failed.");
        std::process::exit(1);
    }

    // serialize and write info, source, and settings to args.output / {}.json. make directories if necessary
    let output_dir = PathBuf::from_str(&args.output)?;
    std::fs::create_dir_all(output_dir.clone())?;
    let info_path = output_dir.join("info.json");
    let source_path = output_dir.join("source.json");
    let settings_path = output_dir.join("settings.json");
    let info_json = serde_json::to_string_pretty(&info)?;
    let source_json = serde_json::to_string_pretty(&source)?;
    let settings_json = serde_json::to_string_pretty(&settings)?;
    std::fs::write(info_path, info_json)?;
    std::fs::write(source_path, source_json)?;
    std::fs::write(settings_path, settings_json)?;

    // delete src/* and test/*
    let src_dir = output_dir.join("src");
    let test_dir = output_dir.join("test");
    let script_dir = output_dir.join("script");
    std::fs::remove_dir_all(src_dir.clone())?;
    std::fs::remove_dir_all(test_dir.clone())?;
    std::fs::remove_dir_all(script_dir.clone())?;
    std::fs::create_dir_all(src_dir)?;
    std::fs::create_dir_all(test_dir)?;
    std::fs::create_dir_all(script_dir)?;

    // rebuild source
    source.write_source_to(&output_dir)?;

    // compile
    compiler::compile(&output_dir, &settings, &info)?;

    Ok(())
}
