#![allow(deprecated)]
use std::{env::home_dir, io::Write};

use clap::Parser;
use eyre::{eyre, OptionExt, Result};
use serde::{Deserialize, Serialize};

const PURPLE_ANSI_COLOR: &str = "\x1b[35m";
const RED_ANSI_COLOR: &str = "\x1b[31m";
const GREEN_ANSI_COLOR: &str = "\x1b[32m";
const RESET_ANSI_COLOR: &str = "\x1b[0m";

#[derive(Debug, Clone, Parser)]
#[clap(
    about = "Display or edit your shadow CLI configuration.",
    override_usage = "heimdall config [OPTIONS]"
)]
pub struct ConfigArgs {
    /// The target key to update.
    #[clap(required = false, default_value = "", hide_default_value = true)]
    key: String,

    /// The value to set the key to.
    #[clap(required = false, default_value = "", hide_default_value = true)]
    value: String,

    /// Whether to enter interactive mode.
    #[clap(long, short)]
    interactive: bool,
}

/// The [`Configuration`] struct represents the configuration of the CLI.
/// TODO: @jon-becker Move this to crates/config
#[derive(Deserialize, Serialize, Debug)]
pub struct Configuration {
    /// The API key to use for Etherscan interactions.
    pub etherscan_api_key: Option<String>,

    /// The URL of the IPFS gateway to use for IPFS interactions.
    pub ipfs_gateway_url: Option<String>,

    /// The API key to use for IPFS interactions.
    pub ipfs_api_key: Option<String>,

    /// The wallet address to use for signing and attestations.
    pub wallet_address: Option<String>,

    /// The name of the contract group to interact with.
    pub contract_group_name: Option<String>,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            etherscan_api_key: None,
            ipfs_gateway_url: None,
            ipfs_api_key: None,
            wallet_address: None,
            contract_group_name: None,
        }
    }
}

#[allow(deprecated)]
impl Configuration {
    /// Returns the current configuration.
    fn load() -> Result<Self> {
        let mut config_path = home_dir().ok_or_eyre("failed to get home directory")?;
        config_path.push(".shadow");
        config_path.push("config.json");

        if !config_path.exists() {
            return Err(eyre!(
                "configuration does not exist at {:?}. If this is your first time using the Shadow CLI, try `shadow config --interactive`",
                config_path
            ));
        }

        let config = std::fs::read_to_string(config_path)?;
        let config: Configuration = serde_json::from_str(&config)?;

        // now load from env, env should override config values
        let env_config = Self::load_from_env()?;
        let config = Configuration {
            etherscan_api_key: env_config.etherscan_api_key.or(config.etherscan_api_key),
            ipfs_gateway_url: env_config.ipfs_gateway_url.or(config.ipfs_gateway_url),
            ipfs_api_key: env_config.ipfs_api_key.or(config.ipfs_api_key),
            wallet_address: env_config.wallet_address.or(config.wallet_address),
            contract_group_name: env_config.contract_group_name.or(config.contract_group_name),
        };

        Ok(config)
    }

    /// Loads configuration from env with envy
    fn load_from_env() -> Result<Self> {
        envy::from_env::<Configuration>().map_err(Into::into)
    }

    /// Saves the configuration to disk.
    fn save(&self) -> Result<()> {
        let mut config_path = home_dir().ok_or_eyre("failed to get home directory")?;
        config_path.push(".shadow");

        // build the directory if it doesn't exist
        if !config_path.exists() {
            std::fs::create_dir_all(&config_path)?;
        }

        config_path.push("config.json");
        let config = serde_json::to_string_pretty(self)?;
        std::fs::write(config_path, config)?;

        Ok(())
    }

    /// Set a value
    fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "etherscan_api_key" => self.etherscan_api_key = Some(value.to_string()),
            "ipfs_gateway_url" => self.ipfs_gateway_url = Some(value.to_string()),
            "ipfs_api_key" => self.ipfs_api_key = Some(value.to_string()),
            "wallet_address" => self.wallet_address = Some(value.to_string()),
            "contract_group_name" => self.contract_group_name = Some(value.to_string()),
            _ => return Err(eyre!("invalid key '{}'", key)),
        };

        self.save()?;

        Ok(())
    }

    /// Starts blocking interactive mode for configuration.
    fn from_interactive() -> Result<Self> {
        let mut config = Configuration::load().unwrap_or_default();
        let input = &mut String::new();

        println!(
            "{PURPLE_ANSI_COLOR}Welcome to the Shadow CLI configuration wizard!{RESET_ANSI_COLOR}\n\nI'll help walk you through configuring the CLI. If you wish to use an existing configuration value, just press enter.\nYou can exit this wizard at any time by pressing `Ctrl+C`.\n",
        );

        // etherscan_api_key
        print!(
            "{GREEN_ANSI_COLOR}1.{RESET_ANSI_COLOR} Set a new Etherscan API key (default: {:?}): ",
            config.etherscan_api_key
        );
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(input)?;
        if !input.trim().is_empty() {
            config.etherscan_api_key = Some(input.trim().to_string());
            input.clear();
        }

        // ipfs_gateway_url
        print!(
            "{GREEN_ANSI_COLOR}2.{RESET_ANSI_COLOR} Set a new IPFS gateway URL (default: {:?}): ",
            config.ipfs_gateway_url
        );
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(input)?;
        if !input.trim().is_empty() {
            config.ipfs_gateway_url = Some(input.trim().to_string());
            input.clear();
        }

        // ipfs_api_key
        print!(
            "{GREEN_ANSI_COLOR}3.{RESET_ANSI_COLOR} Set a new IPFS API key (default: {:?}): ",
            config.ipfs_api_key
        );
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(input)?;
        if !input.trim().is_empty() {
            config.ipfs_api_key = Some(input.trim().to_string());
            input.clear();
        }

        // wallet_address
        print!(
            "{GREEN_ANSI_COLOR}4.{RESET_ANSI_COLOR} Set a new wallet address (default: {:?}): ",
            config.wallet_address
        );
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(input)?;
        if !input.trim().is_empty() {
            config.wallet_address = Some(input.trim().to_string());
            input.clear();
        }

        // contract_group_name
        print!("{GREEN_ANSI_COLOR}5.{RESET_ANSI_COLOR} Set a new contract group name (default: {:?}): ", config.contract_group_name);
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(input)?;
        if !input.trim().is_empty() {
            config.contract_group_name = Some(input.trim().to_string());
            input.clear();
        }

        println!(
            "\n{GREEN_ANSI_COLOR}Configuration set!{RESET_ANSI_COLOR}\n{}",
            serde_json::to_string_pretty(&config)?
        );

        config.save()?;

        Ok(config)
    }
}

/// The `config` command is used to display and edit the current configuration.
/// Note @jon-becker: Not using tracing here because it doesnt look good in interactive mode.
pub(crate) fn config(args: ConfigArgs) -> Result<()> {
    if args.interactive {
        Configuration::from_interactive()?;
        return Ok(());
    }

    if !args.key.is_empty() {
        if !args.value.is_empty() {
            let mut config = Configuration::load()?;
            match config.set(&args.key, &args.value) {
                Ok(_) => {
                    println!(
                        "{GREEN_ANSI_COLOR}Success: {RESET_ANSI_COLOR}'{}' set to '{}'.",
                        args.key, args.value
                    );
                    println!("Configuration: {}\n", serde_json::to_string_pretty(&config)?);
                }
                Err(e) => println!("{RED_ANSI_COLOR}Error: {RESET_ANSI_COLOR}{}", e),
            };
        } else {
            println!("{RED_ANSI_COLOR}Error: {RESET_ANSI_COLOR}use `shadow config <KEY> <VALUE>` to set a key/value pair, or `shadow config --interactive` to enter interactive mode.");
        }
    } else {
        let config = Configuration::load()?;
        println!("Configuration: {}\n", serde_json::to_string_pretty(&config)?);
        println!("{GREEN_ANSI_COLOR}Hint: {RESET_ANSI_COLOR}use `shadow config <KEY> <VALUE>` to set a key/value pair, or `shadow config --interactive` to enter interactive mode.");
    }

    Ok(())
}
