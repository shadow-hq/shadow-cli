#![allow(deprecated)]
use std::{env::home_dir, io::Write};

use crate::constants::{GREEN_ANSI_COLOR, PURPLE_ANSI_COLOR, RESET_ANSI_COLOR};
use eyre::{eyre, OptionExt, Result};
use serde::{Deserialize, Serialize};

/// The [`Configuration`] struct represents the configuration of the CLI.
#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Configuration {
    /// The API key to use for Etherscan interactions.
    pub etherscan_api_key: Option<String>,
    /// The URL of the IPFS gateway to use for IPFS interactions.
    pub ipfs_gateway_url: Option<String>,
    /// The API key to use for IPFS interactions.
    pub pinata_api_key: Option<String>,
    /// The secret API key to use for IPFS interactions.
    pub pinata_secret_api_key: Option<String>,
    /// Your RPC URL.
    pub rpc_url: Option<String>,
}

#[allow(deprecated)]
impl Configuration {
    /// Returns the current configuration.
    pub fn load() -> Result<Self> {
        let mut config_path = home_dir().ok_or_eyre("failed to get home directory")?;
        config_path.push(".shadow");
        config_path.push("config.json");

        if !config_path.exists() {
            // write it
            let config = Configuration::default();
            config.save()?;
        }

        let config = std::fs::read_to_string(config_path)?;
        let config: Configuration = serde_json::from_str(&config)?;

        // now load from env, env should override config values
        let env_config = Self::load_from_env()?;
        let config = Configuration {
            etherscan_api_key: env_config.etherscan_api_key.or(config.etherscan_api_key),
            ipfs_gateway_url: env_config.ipfs_gateway_url.or(config.ipfs_gateway_url),
            pinata_api_key: env_config.pinata_api_key.or(config.pinata_api_key),
            pinata_secret_api_key: env_config
                .pinata_secret_api_key
                .or(config.pinata_secret_api_key),
            rpc_url: env_config.rpc_url.or(config.rpc_url),
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
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "etherscan_api_key" => self.etherscan_api_key = Some(value.to_string()),
            "ipfs_gateway_url" => self.ipfs_gateway_url = Some(value.to_string()),
            "rpc_url" => self.rpc_url = Some(value.to_string()),
            "pinata_api_key" => self.pinata_api_key = Some(value.to_string()),
            "pinata_secret_api_key" => self.pinata_secret_api_key = Some(value.to_string()),
            _ => return Err(eyre!("invalid key '{}'", key)),
        };

        self.save()?;

        Ok(())
    }

    /// Starts blocking interactive mode for configuration.
    pub fn from_interactive() -> Result<Self> {
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
            "{GREEN_ANSI_COLOR}3.{RESET_ANSI_COLOR} Set a new Pinata API key (default: {:?}): ",
            config.pinata_api_key
        );
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(input)?;
        if !input.trim().is_empty() {
            config.pinata_api_key = Some(input.trim().to_string());
            input.clear();
        }

        // ipfs_secret_api_key
        print!(
            "{GREEN_ANSI_COLOR}4.{RESET_ANSI_COLOR} Set a new Pinata secret API key (default: {:?}): ",
            config.pinata_secret_api_key
        );
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(input)?;
        if !input.trim().is_empty() {
            config.pinata_secret_api_key = Some(input.trim().to_string());
            input.clear();
        }

        // rpc_url
        print!(
            "{GREEN_ANSI_COLOR}4.{RESET_ANSI_COLOR} Set a new RPC URL (default: {:?}): ",
            config.rpc_url
        );
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(input)?;
        if !input.trim().is_empty() {
            config.rpc_url = Some(input.trim().to_string());
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
