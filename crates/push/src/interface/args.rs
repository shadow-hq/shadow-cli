use alloy::transports::http::reqwest::Url;
use alloy_chains::NamedChain;
use clap::Parser;
use eyre::{OptionExt, Result};
use revm::primitives::{address, Address};
use serde::Serialize;

/// supported signers enum
#[derive(clap::ValueEnum, Clone, Default, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SignerType {
    /// Use a private key
    #[default]
    PrivateKey,
    /// Use a keystore file
    Keystore,
    /// Use a mnemonic
    Mnemonic,
    /// Use ledger hardware wallet
    Ledger,
    /// Use a Trezor hardware wallet
    Trezor,
    /// Use a Yubikey hardware wallet
    Yubikey,
}

/// supported chains enum
#[derive(clap::ValueEnum, Clone, Default, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SupportedChains {
    /// Base
    #[default]
    Base,
    /// Sepolia (testnet)
    Sepolia,
}

impl From<&SupportedChains> for NamedChain {
    fn from(val: &SupportedChains) -> Self {
        match val {
            SupportedChains::Base => NamedChain::Base,
            SupportedChains::Sepolia => NamedChain::Sepolia,
        }
    }
}

impl SupportedChains {
    /// Get the schema UID for the given chain
    pub fn schema_uid(&self) -> &str {
        match self {
            SupportedChains::Base => {
                "dae982d91ec2b394679937bab01d873f54bbdaef8a483b9b1a55b8edb1bfc988"
            }
            SupportedChains::Sepolia => {
                "dae982d91ec2b394679937bab01d873f54bbdaef8a483b9b1a55b8edb1bfc988"
            }
        }
    }

    /// Get the EAS address for the given chain
    pub fn eas_address(&self) -> Address {
        match self {
            SupportedChains::Base => address!("4200000000000000000000000000000000000021"),
            SupportedChains::Sepolia => address!("C2679fBD37d54388Ce493F1DB75320D236e1815e"),
        }
    }

    /// Get the chain id for the given chain
    pub fn chain_id(&self) -> u64 {
        match self {
            SupportedChains::Base => 8453,
            SupportedChains::Sepolia => 11155111,
        }
    }

    /// Get the public rpc url for the given chain
    pub fn rpc_url(&self) -> Url {
        match self {
            SupportedChains::Base => "https://base-rpc.publicnode.com".parse().expect("valid url"),
            SupportedChains::Sepolia => {
                "https://ethereum-sepolia-rpc.publicnode.com".parse().expect("valid url")
            }
        }
    }

    /// Get the explorer url for the given chain
    pub fn explorer_url(&self) -> String {
        match self {
            SupportedChains::Base => "basescan.org".to_string(),
            SupportedChains::Sepolia => "sepolia.etherscan.io".to_string(),
        }
    }
}

/// Arguments for the `push` subcommand
#[derive(Debug, Clone, Parser)]
#[clap(about = "Compiles and uploads/pins a shadow contract group to IPFS")]
pub struct PushArgs {
    /// The path to the directory in which to initialize the shadow contract group.
    #[clap(short, long, default_value = ".", required = false)]
    pub root: String,

    /// The type of signer you wish to use when attesting.
    #[clap(short, long, default_value = "private-key", required = false)]
    pub signer: SignerType,

    /// Your pinata API key, used to pin the shadow contract group to IPFS.
    #[clap(long, required = false, alias = "ipfs-api-key")]
    pub pinata_api_key: Option<String>,

    /// Your pinata secret API key, used to pin the shadow contract group to IPFS.
    #[clap(long, required = false, alias = "ipfs-secret-api-key")]
    pub pinata_secret_api_key: Option<String>,

    /// Your preferred IPFS gateway, used when displaying the IPFS URL.
    #[clap(
        long,
        required = false,
        default_value = "https://gateway.pinata.cloud/ipfs/",
        hide_default_value = true
    )]
    pub ipfs_gateway_url: String,

    /// The chain to use when attesting.
    #[clap(short, long, default_value = "base", required = false)]
    pub chain: SupportedChains,
}

impl PushArgs {
    /// Validates the configuration arguments.
    pub fn validate(&self) -> Result<()> {
        let _ = self.pinata_api_key.as_ref().ok_or_eyre(
               "IPFS API key must be set. Use the --pinata-api-key flag or set the IPFS_API_KEY environment variable.")?;
        let _ = self.pinata_secret_api_key.as_ref().ok_or_eyre(
               "IPFS secret API key must be set. Use the --pinata-secret-api-key flag or set the IPFS_SECRET_API_KEY environment variable.")?;

        Ok(())
    }
}
