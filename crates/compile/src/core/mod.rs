use std::{env::temp_dir, os::unix::process::CommandExt, path::PathBuf, str::FromStr};

use crate::CompileArgs;
use alloy_chains::{Chain, NamedChain};
use eyre::{eyre, OptionExt, Result};
use foundry_block_explorers::Client;
use shadow_common::{compiler, ShadowContractInfo, ShadowContractSettings, ShadowContractSource};
use tracing::{error, info, trace, warn};
use which::Path;

/// The `compile` subcommand. Compiles a shadowed contract with the original contract settings.
/// TODO: @jon-becker clean this up w/ helpers rather than a single function
/// TODO: @jon-becker --force flag
pub async fn compile(args: CompileArgs) -> Result<()> {
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

    // compile
    compiler::compile(&root_dir, &settings, &info)?;

    Ok(())
}
