use std::path::{Path, PathBuf};

use alloy::primitives::Address;
use alloy_chains::Chain;
use eyre::Result;
use foundry_block_explorers::contract::{ContractCreationData, ContractMetadata};
use foundry_compilers::artifacts::{RelativeRemapping, Remapping};
use revm::primitives::B256;
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
    /// The deployment transaction hash
    #[serde(rename = "deploymentTransactionHash")]
    pub deployment_transaction_hash: B256,
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
            network: chain.named().map(|n| n.to_string()).unwrap_or_else(|| "unknown".to_string()),
            chain_id: chain.id(),
            source: "etherscan".to_string(),
            unique_events: 0,
            deployment_transaction_hash: creation_data.transaction_hash,
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
    /// Remappings used to compile the contract
    pub remappings: Vec<RelativeRemapping>,
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
    ///
    /// Much of this code (for the reorg logic) is taken from `forge clone` \
    /// https://github.com/foundry-rs/foundry/blob/master/crates/forge/bin/cmd/clone.rs
    pub fn new(metadata: &ContractMetadata) -> Result<Self> {
        let metadata = metadata.items.clone().remove(0);
        let source_tree = metadata.source_tree();

        // get cwd
        let root = tempdir::TempDir::new("clone")?.into_path();
        let raw_dir = root.join("raw");
        let lib_dir = root.join("lib");
        let src_dir = root.join("src");

        let mut remappings = vec![Remapping {
            context: None,
            name: "forge-std".to_string(),
            path: root.join("lib/forge-std/src").to_string_lossy().to_string(),
        }];

        // ensure all directories are created
        std::fs::create_dir_all(&lib_dir)?;
        std::fs::create_dir_all(&src_dir)?;

        source_tree.write_to(&raw_dir).map_err(|e| eyre::eyre!("failed to dump sources: {}", e))?;

        // check if the source needs reorginazation
        let needs_reorg = std::fs::read_dir(raw_dir.join(&metadata.contract_name))?.all(|e| {
            let Ok(e) = e else { return false };
            let folder_name = e.file_name();
            folder_name == "src" ||
                folder_name == "lib" ||
                folder_name == "contracts" ||
                folder_name == "hardhat" ||
                folder_name == "forge-std" ||
                folder_name.to_string_lossy().starts_with('@')
        });

        // move source files
        for entry in std::fs::read_dir(raw_dir.join(&metadata.contract_name))? {
            let entry = entry?;
            let folder_name = entry.file_name();
            // special handling when we need to re-organize the directories: we flatten them.
            if needs_reorg {
                if folder_name == "contracts" || folder_name == "src" || folder_name == "lib" {
                    // move all sub folders in contracts to src or lib
                    let new_dir = if folder_name == "lib" { &lib_dir } else { &src_dir };
                    for e in std::fs::read_dir(entry.path())? {
                        let e = e?;
                        let dest = new_dir.join(e.file_name());
                        eyre::ensure!(
                            !Path::exists(&dest),
                            "destination already exists: {:?}",
                            dest
                        );
                        std::fs::rename(e.path(), &dest)?;
                        remappings.push(Remapping {
                            context: None,
                            name: format!(
                                "{}/{}",
                                folder_name.to_string_lossy(),
                                e.file_name().to_string_lossy()
                            ),
                            path: dest.to_string_lossy().to_string(),
                        });
                    }
                } else {
                    assert!(
                        folder_name == "hardhat" ||
                            folder_name == "forge-std" ||
                            folder_name.to_string_lossy().starts_with('@')
                    );
                    // move these other folders to lib
                    let dest = lib_dir.join(&folder_name);
                    if folder_name == "forge-std" {
                        // let's use the provided forge-std directory
                        std::fs::remove_dir_all(&dest)?;
                    }
                    eyre::ensure!(!Path::exists(&dest), "destination already exists: {:?}", dest);
                    std::fs::rename(entry.path(), &dest)?;
                    remappings.push(Remapping {
                        context: None,
                        name: folder_name.to_string_lossy().to_string(),
                        path: dest.to_string_lossy().to_string(),
                    });
                }
            } else {
                // directly move the all folders into src
                let dest = src_dir.join(&folder_name);
                eyre::ensure!(!Path::exists(&dest), "destination already exists: {:?}", dest);
                std::fs::rename(entry.path(), &dest)?;
                if folder_name != "src" {
                    remappings.push(Remapping {
                        context: None,
                        name: folder_name.to_string_lossy().to_string(),
                        path: dest.to_string_lossy().to_string(),
                    });
                }
            }
        }

        // delete the raw directory
        std::fs::remove_dir_all(raw_dir)?;

        // add remappings in the metedata
        for mut r in metadata.settings()?.remappings {
            if needs_reorg {
                // we should update its remapped path in the same way as we dump sources
                // i.e., remove prefix `contracts` (if any) and add prefix `src`
                let new_path = if r.path.starts_with("contracts") {
                    PathBuf::from("src").join(PathBuf::from(&r.path).strip_prefix("contracts")?)
                } else if r.path.starts_with('@') ||
                    r.path.starts_with("hardhat/") ||
                    r.path.starts_with("forge-std/")
                {
                    PathBuf::from("lib").join(PathBuf::from(&r.path))
                } else {
                    PathBuf::from(&r.path)
                };
                r.path = new_path.to_string_lossy().to_string();
            }

            remappings.push(r);
        }

        Ok(Self {
            compiler_version: metadata.compiler_version.clone(),
            language: if metadata.is_vyper() {
                "Vyper".to_string()
            } else {
                "Solidity".to_string()
            },
            remappings: remappings.into_iter().map(|r| r.into_relative(&root)).collect(),
            contract_files: walkdir::WalkDir::new(&root)
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
                        file_name: file_path.strip_prefix(&root)?.to_string_lossy().to_string(),
                        content: contents,
                    })
                })
                .collect::<Result<Vec<ShadowContractSourceFile>>>()?,
        })
    }

    /// Builds the source directory
    pub fn write_source_to(&self, src_dir: &Path) -> Result<()> {
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

        // write remappings.txt
        let remappings_path = src_dir.join("remappings.txt");
        let remappings = self
            .remappings
            .iter()
            .map(|r| {
                format!(
                    "{}{}={}",
                    r.name,
                    if !r.name.to_string().ends_with('/') { "/" } else { "" },
                    r.path.original().display()
                )
            })
            .collect::<Vec<String>>()
            .join("\n");
        std::fs::write(remappings_path, remappings)?;

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
            remappings: vec![],
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
    /// Via IR
    #[serde(rename = "viaIr")]
    pub via_ir: bool,
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
            evm_version: metadata.evm_version().ok().flatten().unwrap_or_default().to_string(),
            via_ir: metadata.settings().map(|s| s.via_ir).ok().flatten().unwrap_or(false),
        }
    }

    /// Writes the settings to a `foundry.toml` configuration file
    /// TODO @jon-becker: Eventually use the toml crate for this
    pub fn generate_config(&self, src_root: &Path) -> Result<()> {
        let config_path = src_root.join("foundry.toml");
        let config = format!(
            "[profile.default]\nsrc = \"src\"\nout = \"out\"\nlibs = [\"lib\"]\noptimizer = {}\noptimizer_runs = {}\nbytecode_hash = \"none\"\nsolc_version = \"{}\"\nevm_version = \"{}\"\nvia_ir = {}",
            self.optimizer.enabled,
            self.optimizer.runs,
            self.compiler_version.strip_prefix('v').unwrap_or(&self.compiler_version),
            self.evm_version,
            self.via_ir
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
