use std::{
    io::Write,
    path::{Path, PathBuf},
};

use alloy::primitives::Address;
use chrono::{DateTime, Utc};
use eyre::{bail, OptionExt, Result};
use futures::future::try_join_all;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::{compiler, ShadowContractInfo, ShadowContractSettings, ShadowContractSource};

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

impl ShadowContractEntry {
    /// Compiles the contract that this entry references
    pub async fn compile(&self, rpc_url: &str, root: &Path, output: &Path) -> Result<()> {
        let start_time = std::time::Instant::now();

        // build paths
        let contract_path =
            root.join(self.chain_id.to_string()).join(self.address.to_string().to_lowercase());
        let contract_info_path = contract_path.join("info.json");
        let contract_settings_path = contract_path.join("settings.json");
        let contract_src_path = contract_path.join("src");
        let contract_original_source_path = contract_path.join("original.json");

        let contract_output_path =
            output.join(self.chain_id.to_string()).join(self.address.to_string().to_lowercase());
        let out_bytecode_file = contract_output_path.join("bytecode.hex");
        let out_abi_file = contract_output_path.join("abi.json");
        let out_settings_file = contract_output_path.join("settings.json");
        let out_contract_info_file = contract_output_path.join("info.json");
        let out_source_file = contract_output_path.join("source.json");
        let out_original_file = contract_output_path.join("original.json");

        // ensure output directory exists
        std::fs::create_dir_all(&contract_output_path)?;

        // load contract info and compiler settings
        let mut contract_info = ShadowContractInfo::from_path(&contract_info_path)?;
        let contract_settings = ShadowContractSettings::from_path(&contract_settings_path)?;

        debug!(
            "Compiling contract {} ({}:{}) with {}...",
            contract_info.name, self.chain_id, self.address, contract_settings.compiler_version
        );

        // compile the contract
        let output =
            compiler::compile(rpc_url, &contract_path, &contract_settings, &contract_info).await?;

        debug!("Compiled {} successfully in {:?}", contract_info.name, start_time.elapsed());

        // update contract info
        contract_info.unique_events = output.abi.events.len() as u64;
        let source = ShadowContractSource::from_path(&contract_src_path, &contract_settings)?;

        // write output files
        std::fs::write(out_bytecode_file, format!("0x{}", hex::encode(&output.bytecode)))?;
        std::fs::write(out_abi_file, serde_json::to_string(&output.abi)?)?;
        std::fs::write(out_settings_file, serde_json::to_string(&contract_settings)?)?;
        std::fs::write(out_contract_info_file, serde_json::to_string(&contract_info)?)?;
        std::fs::write(contract_info_path, serde_json::to_string_pretty(&contract_info)?)?; // update original contract info
        std::fs::write(out_source_file, serde_json::to_string(&source)?)?;
        std::fs::copy(contract_original_source_path, out_original_file)?;

        Ok(())
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

        info.root.clone_from(path);

        Ok(info)
    }

    /// Writes the folder structure of the contract group to the provided path.
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
            .filter(|e| e.path() != self.root.join("info.json"))
            .map(|e| {
                let contract_info: ShadowContractInfo =
                    serde_json::from_str(&std::fs::read_to_string(e.path())?)?;

                Ok(contract_info.into())
            })
            .collect::<Result<Vec<ShadowContractEntry>>>()?;

        // update creation date
        self.creation_date = Utc::now();

        // write the updated info.json
        let info_file = self.root.join("info.json");
        let info_json = serde_json::to_string_pretty(self)?;
        std::fs::write(info_file, info_json)?;

        // update readme
        self.readme = std::fs::read_to_string(self.root.join("README.md"))?;

        Ok(())
    }

    /// Validates that the group information is ready for pinning to IPFS
    pub fn validate(&mut self) -> Result<()> {
        // group must have a display name
        if &self.display_name == "Unnamed Contract Group" {
            let display_name = prompt("Enter a display name for this contract group: ")?
                .ok_or_eyre("no display name provided")?;
            self.display_name = display_name
        }

        // creator must exist
        if self.creator.is_none() {
            let creator = prompt("Enter the creator address of this contract group: ")?
                .ok_or_eyre("no creator address provided")?;
            self.creator = Some(creator.parse()?);
        }

        // if readme is unchanged, prompt user to update it
        let readme_file = self.root.join("README.md");
        let readme = std::fs::read_to_string(readme_file)?;
        if readme == DEFAULT_README {
            let skip_readme = prompt("You have not updated the README.md file for your contract group. Would you like to skip this step? (y/N)")?
                .unwrap_or_else(|| "n".to_string())
                .to_lowercase();

            if skip_readme != "y" {
                bail!("Please update the README.md file for your contract group");
            }
        }

        // update info.json
        let info_file = self.root.join("info.json");
        let info_json = serde_json::to_string_pretty(self)?;
        std::fs::write(info_file, info_json)?;

        Ok(())
    }

    /// Prepares the contract group for pinning to IPFS. Compiles all shadow contracts
    /// in the group and generates the proper folder structure which will be pinned
    /// to IPFS.
    pub async fn prepare(&mut self, rpc_url: &str) -> Result<PathBuf> {
        // re-scan the contracts directory for new contracts
        let _ = &self.update_contracts()?;

        // create an `out` directory in the group's root
        let out_dir = self.root.join("out");
        std::fs::remove_dir_all(&out_dir).ok();
        std::fs::create_dir_all(&out_dir)?;

        // copy `info.json` and `README.md` to the out directory, since this will be pinned
        let out_folder = self.write_folder_structure(out_dir)?;

        // we need to compile each contract in the group. We can do this in parallel w/ rayon
        info!("compiling {} shadow contracts", self.contracts.len());
        let compile_futures = self
            .contracts
            .par_iter()
            .map(|contract| contract.compile(rpc_url, &self.root, &out_folder))
            .collect::<Vec<_>>();

        try_join_all(compile_futures).await?;

        info!("compiled all shadow contracts successfully");

        Ok(out_folder)
    }
}

/// Prompt the user for input w/ pretty colors :D
fn prompt(text: &str) -> Result<Option<String>> {
    let mut input = String::new();
    const YELLOW_ANSI_CODE: &str = "\u{001b}[33m";
    const LIGHT_GRAY_ANSI_CODE: &str = "\u{001b}[90m";
    const RESET_ANSI_CODE: &str = "\u{001b}[0m";

    print!(
        "{LIGHT_GRAY_ANSI_CODE}{}  {YELLOW_ANSI_CODE}WARN{RESET_ANSI_CODE} {}",
        // include microsecond precision
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
        text,
    );

    std::io::stdout().flush().unwrap();
    std::io::stdin().read_line(&mut input)?;
    if !input.trim().is_empty() {
        return Ok(Some(input.trim().to_string()));
    }

    Ok(None)
}
