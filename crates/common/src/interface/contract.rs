use std::path::PathBuf;

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
    /// Unique events not part of the original contract
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
            unique_events: 0, // This is directly from mainnet, so there are no additional non-canonical events
        }
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
    pub fn write_source_to(&self, src_root: &PathBuf) -> Result<()> {
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
    /// The optimization used
    #[serde(rename = "optimizationUsed")]
    pub optimization_used: u64,
    /// The number of runs
    pub runs: u64,
    /// The constructor arguments
    #[serde(rename = "constructorArguments")]
    pub constructor_arguments: Vec<u8>,
    /// The EVM version
    #[serde(rename = "evmVersion")]
    pub evm_version: String,
    /// The library
    pub library: String,
    /// The license type
    #[serde(rename = "licenseType")]
    pub license_type: String,
    /// The proxy
    pub proxy: u64,
    /// The implementation
    pub implementation: Option<Address>,
    /// The swarm source
    #[serde(rename = "swarmSource")]
    pub swarm_source: String,
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
            optimization_used: metadata.optimization_used,
            runs: metadata.runs,
            constructor_arguments: metadata.constructor_arguments.to_vec(),
            evm_version: metadata.evm_version.clone(),
            library: metadata.library.clone(),
            license_type: metadata.license_type.clone(),
            proxy: metadata.proxy,
            implementation: metadata.implementation.clone(),
            swarm_source: metadata.swarm_source.clone(),
        }
    }
}
