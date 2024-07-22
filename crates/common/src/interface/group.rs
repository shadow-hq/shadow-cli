use std::path::PathBuf;

use alloy::primitives::Address;
use alloy_chains::Chain;
use chrono::{DateTime, Utc};
use eyre::Result;
use foundry_block_explorers::contract::{ContractCreationData, ContractMetadata};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ShadowContractInfo;

/// Contains the initial, default README.md file for a contract group
pub const DEFAULT_README: &str = include_str!("../../templates/README.md");

/// Contract group information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowContractGroupInfo {
    /// The display name of the contract group
    #[serde(rename = "displayName")]
    pub display_name: String,
    /// The address of the creator of the contract group
    pub creator: Option<Address>,
    /// The date the contract group was created
    #[serde(rename = "creationDate")]
    pub creation_date: DateTime<Utc>,
    /// A list of contracts in the contract group
    pub contracts: Vec<ShadowContractEntry>,
    /// Path to the contract group root
    #[serde(skip)]
    root: PathBuf,
}

/// A single contract in a contract group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowContractEntry {
    /// The address of the contract
    pub address: Address,
    /// The chain id that the contract is deployed on
    pub chain_id: u64,
}

impl From<ShadowContractInfo> for ShadowContractEntry {
    fn from(info: ShadowContractInfo) -> Self {
        Self { address: info.address, chain_id: info.chain_id }
    }
}

impl Default for ShadowContractGroupInfo {
    fn default() -> Self {
        Self {
            display_name: "Unnamed Contract Group".to_string(),
            creator: None,
            creation_date: Utc::now(),
            contracts: vec![],
            root: PathBuf::new(),
        }
    }
}

impl ShadowContractGroupInfo {
    /// Try to create a new instance of [`ShadowContractGroupInfo`] from the provided
    /// path. Assumes the path is a directory containing a `info.json` file.
    pub fn from_path(path: &PathBuf) -> Result<Self> {
        let info_file = path.join("info.json");
        let info_json = std::fs::read_to_string(info_file)?;
        let mut info: Self = serde_json::from_str(&info_json)?;

        info.root = path.clone();

        Ok(info)
    }

    /// Writes the folder structure of the contract group to the provided path
    /// Returns the path to the created folder
    pub fn write_folder_structure(&self, parent: PathBuf) -> Result<PathBuf> {
        // parent/ContractGroup_06_20_2024_12_00
        let formatted_date = self.creation_date.format("%m_%d_%Y_%H_%M");
        let group_folder = parent.join(format!("ContractGroup_{}", formatted_date));
        std::fs::create_dir_all(&group_folder)?;

        // write to group_folder/info.json
        let info_file = group_folder.join("info.json");
        let info_json = serde_json::to_string_pretty(self)?;
        std::fs::write(info_file, info_json)?;

        // write to group_folder/README.md
        let readme_file = group_folder.join("README.md");
        std::fs::write(readme_file, DEFAULT_README)?;

        Ok(group_folder)
    }

    /// Updates the group's contracts by scanning the contracts directory
    /// for new contracts
    pub fn update_contracts(&mut self) -> Result<()> {
        // walk the directory recursively. We only care about `info.json` files
        self.contracts = walkdir::WalkDir::new(&self.root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with("info.json"))
            .into_iter()
            .filter(|e| e.path() != self.root.join("info.json"))
            .map(|e| {
                let contract_info: ShadowContractInfo =
                    serde_json::from_str(&std::fs::read_to_string(e.path())?)?;

                Ok(contract_info.into())
            })
            .collect::<Result<Vec<ShadowContractEntry>>>()?;

        // write the updated info.json
        let info_file = self.root.join("info.json");
        let info_json = serde_json::to_string_pretty(self)?;
        std::fs::write(info_file, info_json)?;

        Ok(())
    }

    /// Prepares the contract group for pinning to IPFS
    pub fn prepare(&self) -> Result<()> {
        // create an `out` directory in the group's root
        let out_dir = self.root.join("out");
        std::fs::create_dir_all(&out_dir)?;

        Ok(())
    }
}
