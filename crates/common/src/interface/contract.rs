use std::path::{Path, PathBuf};

use alloy::primitives::Address;
use alloy_chains::Chain;
use eyre::Result;
use foundry_block_explorers::contract::{ContractCreationData, ContractMetadata};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Contract information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowContractInfo {
    /// The address of the contract
    pub address: Address,
    /// The deployer of the contract
    #[serde(rename = "contractDeployer")]
    pub contract_deployer: Address,
    /// The name of the contract
    pub name: String,
    /// The network the contract is deployed on
    pub network: String,
    /// The chain ID
    #[serde(rename = "chainId")]
    pub chain_id: u64,
    /// Source of the contract information
    pub source: String,
    /// Number of unique events emitted by the contract
    #[serde(rename = "uniqueEvents")]
    pub unique_events: u64,
}

impl ShadowContractInfo {
    /// Creates a new instance of [`ShadowContractInfo`] from the provided
    /// [`ContractMetadata`] and [`ContractCreationData`]
    pub fn new(
        chain: &Chain,
        metadata: &ContractMetadata,
        creation_data: &ContractCreationData,
    ) -> Self {
        Self {
            address: creation_data.contract_address,
            contract_deployer: creation_data.contract_creator,
            name: metadata.items.first().expect("no metadata found").contract_name.clone(),
            network: chain.named().expect("invalid chain").to_string(),
            chain_id: chain.id(),
            source: "etherscan".to_string(),
            unique_events: 0,
        }
    }

    /// Creates a new instance of [`ShadowContractInfo`] from the provided
    /// path to an info.json file
    pub fn from_path(path: &PathBuf) -> Result<Self> {
        let info = std::fs::read_to_string(path)?;
        let info: Value = serde_json::from_str(&info)?;
        Ok(serde_json::from_value(info)?)
    }
}

/// Shadow contract source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowContractSource {
    /// The compiler version used to compile the contract
    #[serde(rename = "compilerVersion")]
    pub compiler_version: String,
    /// The language used to write the contract
    pub language: String,
    /// The source code of the contract
    #[serde(rename = "contractFiles")]
    pub contract_files: Vec<ShadowContractSourceFile>,
}

/// Shadow contract source file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowContractSourceFile {
    /// The name of the file
    #[serde(rename = "fileName")]
    pub file_name: String,
    /// The content of the file
    pub content: String,
}

impl ShadowContractSource {
    /// Creates a new instance of [`ShadowContractSource`] from the provided
    /// [`ContractMetadata`]
    pub fn new(metadata: &ContractMetadata) -> Self {
        Self {
            compiler_version: metadata
                .items
                .first()
                .expect("no metadata found")
                .compiler_version
                .clone(),
            language: if metadata.items.iter().any(|i| i.compiler_version.starts_with("vyper")) {
                "Vyper".to_string()
            } else {
                "Solidity".to_string()
            },
            contract_files: metadata
                .source_tree()
                .entries
                .iter()
                .map(|e| ShadowContractSourceFile {
                    file_name: e.path.to_str().expect("invalid path").to_string(),
                    content: e.contents.clone(),
                })
                .collect(),
        }
    }

    /// Builds the source directory
    pub fn write_source_to(&self, src_root: &Path) -> Result<()> {
        // create the source directory
        let src_dir = src_root.join("src");
        std::fs::create_dir_all(&src_dir)?;

        // write the source files
        for file in self.contract_files.iter() {
            // if the file name doesnt have .sol or .vy extension, add it
            let file_name = if file.file_name.ends_with(".sol") || file.file_name.ends_with(".vy") {
                file.file_name.clone()
            } else if self.language == "Vyper" {
                format!("{}.vy", file.file_name)
            } else {
                format!("{}.sol", file.file_name)
            };
            let file_path = src_dir.join(file_name);

            // create the parent directory if it doesn't exist
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&file_path, &file.content)?;
        }

        Ok(())
    }

    /// Creates a new instance of [`ShadowContractSource`] from the provided
    /// path to /src directory and contract settings
    pub fn from_path(path: &PathBuf, contract_settings: &ShadowContractSettings) -> Result<Self> {
        Ok(Self {
            // we can determine the language based on the compiler version
            language: if contract_settings.compiler_version.starts_with("vyper") {
                "Vyper".to_string()
            } else {
                "Solidity".to_string()
            },
            compiler_version: contract_settings.compiler_version.to_owned(),
            // walk the contract directory and collect all .sol and .vy files
            contract_files: walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    (e.file_name().to_string_lossy().ends_with(".sol") ||
                        e.file_name().to_string_lossy().ends_with(".vy")) &&
                        e.file_type().is_file()
                })
                .map(|e| {
                    let file_path = e.path();
                    let contents = std::fs::read_to_string(file_path)?;

                    Ok(ShadowContractSourceFile {
                        file_name: file_path.strip_prefix(path)?.to_string_lossy().to_string(),
                        content: contents,
                    })
                })
                .collect::<Result<Vec<ShadowContractSourceFile>>>()?,
        })
    }
}

/// Shadow contract settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowContractSettings {
    /// The optimizer settings
    pub optimizer: ShadowOptimizerSettings,
    /// The output selection settings
    /// Note: this will be the same for all contracts
    #[serde(rename = "outputSelection")]
    pub output_selection: Value,
    /// The libraries used by the contract
    pub libraries: Value,
    /// The compiler version used to compile the contract
    #[serde(rename = "compilerVersion")]
    pub compiler_version: String,
    /// The constructor arguments
    #[serde(rename = "constructorArguments")]
    pub constructor_arguments: Vec<u8>,
    /// The EVM version
    #[serde(rename = "evmVersion")]
    pub evm_version: String,
}

/// Optimizer settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowOptimizerSettings {
    /// Whether the optimizer is enabled
    pub enabled: bool,
    /// The number of runs
    pub runs: u64,
}

impl ShadowContractSettings {
    /// Creates a new instance of [`ShadowContractSettings`] from the provided
    /// [`ContractMetadata`]
    pub fn new(metadata: &ContractMetadata) -> Self {
        let metadata = metadata.items.first().expect("no metadata found");
        Self {
            optimizer: ShadowOptimizerSettings {
                enabled: metadata.optimization_used > 0,
                runs: metadata.runs,
            },
            output_selection: serde_json::json!({
                "*": {
                    "*": [
                        "evm.bytecode",
                        "evm.deployedBytecode",
                        "devdoc",
                        "userdoc",
                        "metadata",
                        "abi"
                    ]
                }
            }),
            libraries: serde_json::json!({}),
            compiler_version: metadata.compiler_version.clone(),
            constructor_arguments: metadata.constructor_arguments.to_vec(),
            evm_version: metadata.evm_version.clone(),
        }
    }

    /// Writes the settings to a `foundry.toml` configuration file
    /// TODO @jon-becker: Eventually use the toml crate for this
    pub fn generate_config(&self, src_root: &Path) -> Result<()> {
        let config_path = src_root.join("foundry.toml");
        let config = format!(
            "[profile.default]\nsrc = \"src\"\nout = \"out\"\nlibs = [\"lib\"]\noptimizer = {}\noptimizer_runs = {}\nbytecode_hash = \"none\"\nsolc_version = \"{}\"",
            self.optimizer.enabled,
            self.optimizer.runs,
            self.compiler_version.strip_prefix('v').unwrap_or(&self.compiler_version)
        );

        // overwrite `foundry.toml` if it already exists
        std::fs::write(config_path, config)?;

        Ok(())
    }

    /// Creates a new instance of [`ShadowContractSettings`] from the provided
    /// settings.json file
    pub fn from_path(settings_file: &PathBuf) -> Result<Self> {
        let settings = std::fs::read_to_string(settings_file)?;
        let settings: ShadowContractSettings = serde_json::from_str(&settings)?;
        Ok(settings)
    }
}
