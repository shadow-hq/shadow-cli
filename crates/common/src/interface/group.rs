use std::{env::temp_dir, path::PathBuf};

use alloy::primitives::Address;
use alloy_chains::Chain;
use chrono::{DateTime, Utc};
use eyre::Result;
use foundry_block_explorers::contract::{ContractCreationData, ContractMetadata};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, error, info};

use crate::{
    compiler, ShadowContractInfo, ShadowContractSettings, ShadowContractSource,
    ShadowContractSourceFile,
};

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
    /// The contract group's README.md file
    #[serde(skip)]
    readme: String,
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
            readme: DEFAULT_README.to_string(),
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
        std::fs::write(readme_file, self.readme.clone())?;

        Ok(group_folder)
    }

    /// Updates the group's contracts by scanning the contracts directory
    /// for new contracts
    pub fn update_contracts(&mut self) -> Result<()> {
        // walk the directory recursively. We only care about `info.json` files
        self.contracts = walkdir::WalkDir::new(&self.root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| !e.path().starts_with(self.root.join("out")))
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

        // update readme
        self.readme = std::fs::read_to_string(self.root.join("README.md"))?;

        Ok(())
    }

    /// Prepares the contract group for pinning to IPFS
    pub fn prepare(&self) -> Result<PathBuf> {
        // create an `out` directory in the group's root
        let out_dir = self.root.join("out");
        if out_dir.exists() {
            std::fs::remove_dir_all(&out_dir)?;
        }
        std::fs::create_dir_all(&out_dir)?;

        // Copy the Folder structure to the out directory
        let out_folder = self.write_folder_structure(out_dir)?;

        // We need to compile each contract in the group. We can do this in parallel w/ rayon
        info!("Compiling {} shadow contracts", self.contracts.len());
        self.contracts
            .par_iter()
            .map(|contract| {
                let start_time = std::time::Instant::now();
                debug!("Compiling {} ({})", contract.address, contract.chain_id);

                let contract_path = self
                    .root
                    .join(contract.chain_id.to_string())
                    .join(contract.address.to_string());
                let contract_info_path = contract_path.join("info.json");
                let contract_settings_path = contract_path.join("settings.json");

                let mut contract_info = ShadowContractInfo::from_path(&contract_info_path)?;
                let contract_settings = ShadowContractSettings::from_path(&contract_settings_path)?;

                let contract_output_path = out_folder
                    .join(contract.chain_id.to_string())
                    .join(contract.address.to_string());
                std::fs::create_dir_all(&contract_output_path)?;

                let output = compiler::compile(&contract_path, &contract_settings, &contract_info)?;

                debug!("Compiled {} in {}ms", contract.address, start_time.elapsed().as_millis());

                // write bytecode to `contract_output_path/bytecode.hex`
                let out_bytecode_file = contract_output_path.join("bytecode.hex");
                std::fs::write(out_bytecode_file, format!("0x{}", hex::encode(&output.bytecode)))?;

                // write abi to `contract_output_path/abi.json`
                let out_abi_file = contract_output_path.join("abi.json");
                std::fs::write(out_abi_file, serde_json::to_string(&output.abi)?)?;

                // write settings to `contract_output_path/settings.json`
                let out_settings_file = contract_output_path.join("settings.json");
                std::fs::write(out_settings_file, serde_json::to_string(&contract_settings)?)?;

                // update event count in contract_info
                contract_info.unique_events = output.abi.events.len() as u64;
                let out_contract_info_file = contract_output_path.join("info.json");
                std::fs::write(out_contract_info_file, serde_json::to_string(&contract_info)?)?;
                std::fs::write(contract_info_path, serde_json::to_string_pretty(&contract_info)?)?; // update original contract info

                // rebuild source
                let src_path = contract_path.join("src");
                let out_source_file = contract_output_path.join("source.json");
                let source = ShadowContractSource {
                    language: if contract_settings.compiler_version.starts_with("vyper") {
                        "Vyper".to_string()
                    } else {
                        "Solidity".to_string()
                    },
                    compiler_version: contract_settings.compiler_version,
                    // walk the contract directory and add all .sol / .vy files
                    contract_files: walkdir::WalkDir::new(&src_path)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| {
                            (e.file_name().to_string_lossy().ends_with(".sol") ||
                                e.file_name().to_string_lossy().ends_with(".vy")) &&
                                e.file_type().is_file()
                        })
                        .map(|e| {
                            let path = e.path();
                            let contents = std::fs::read_to_string(path)?;
                            Ok(ShadowContractSourceFile {
                                file_name: // entire path, but strip everything before src/
                                    path.strip_prefix(&src_path)?.to_string_lossy().to_string(),
                                content:contents,
                            })
                        })
                        .collect::<Result<Vec<ShadowContractSourceFile>>>()?,
                };
                std::fs::write(out_source_file, serde_json::to_string(&source)?)?;

                Ok::<(), eyre::Report>(())
            })
            .collect::<Result<Vec<()>>>()?;

        Ok(out_folder)
    }
}
