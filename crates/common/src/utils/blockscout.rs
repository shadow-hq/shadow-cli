use std::collections::HashMap;

use eyre::{OptionExt, Result};
use foundry_block_explorers::contract::{
    ContractCreationData, ContractMetadata, Metadata, SourceCodeEntry, SourceCodeLanguage,
    SourceCodeMetadata,
};
use hex::FromHex;
use revm::primitives::Address;
use serde_json::Value;

/// Blockscout API client
#[derive(Clone, Debug)]
pub struct Client {
    /// Client that executes HTTP requests
    client: reqwest::Client,
    /// The base URL of the Blockscout API
    base_url: String,
}

impl Client {
    /// Creates a new Blockscout API client
    pub fn new(base_url: &str) -> Self {
        Self { client: reqwest::Client::new(), base_url: base_url.to_string() }
    }

    /// Fetches a contract's verified source code and its metadata.
    pub async fn contract_source_code(&self, address: Address) -> Result<ContractMetadata> {
        let url =
            format!("{}/api/v2/smart-contracts/{}", self.base_url.trim_end_matches('/'), address);

        let response = self.client.get(&url).send().await?;
        let response = response.json::<Value>().await?;

        let mut sources = response
            .get("additional_sources")
            .ok_or_eyre("no additional sources")?
            .as_array()
            .ok_or_eyre("invalid additional sources")?
            .iter()
            .map(|source| {
                let file_path = source
                    .get("file_path")
                    .ok_or_eyre("no file_path")?
                    .as_str()
                    .ok_or_eyre("invalid file_path")?;
                let content = source
                    .get("source_code")
                    .ok_or_eyre("no source_code")?
                    .as_str()
                    .ok_or_eyre("invalid source_code")?;

                Ok((file_path.to_string(), SourceCodeEntry { content: content.to_string() }))
            })
            .collect::<Result<HashMap<_, _>>>()?;
        sources.insert(
            response
                .get("file_path")
                .ok_or_eyre("no file_path")?
                .as_str()
                .ok_or_eyre("invalid file_path")?
                .to_string(),
            SourceCodeEntry {
                content: response
                    .get("source_code")
                    .ok_or_eyre("no source_code")?
                    .as_str()
                    .ok_or_eyre("invalid source_code")?
                    .to_string(),
            },
        );

        Ok(ContractMetadata {
            items: vec![Metadata {
                source_code: SourceCodeMetadata::Metadata {
                    language: Some(
                        if response
                            .get("language")
                            .ok_or_eyre("no language")?
                            .as_str()
                            .ok_or_eyre("invalid language value")?
                            .to_lowercase() ==
                            "solidity"
                        {
                            SourceCodeLanguage::Solidity
                        } else {
                            SourceCodeLanguage::Vyper
                        },
                    ),
                    sources,
                    settings: Some(
                        response
                            .get("compiler_settings")
                            .ok_or_eyre("no compiler settings")?
                            .clone(),
                    ),
                },
                abi: serde_json::to_string(response.get("abi").ok_or_eyre("no abi")?)?,
                contract_name: response
                    .get("name")
                    .ok_or_eyre("no name")?
                    .as_str()
                    .ok_or_eyre("invalid name")?
                    .to_string(),
                compiler_version: response
                    .get("compiler_version")
                    .ok_or_eyre("no compiler version")?
                    .as_str()
                    .ok_or_eyre("invalid compiler version")?
                    .to_string(),
                optimization_used: if response
                    .get("optimization_enabled")
                    .ok_or_eyre("no optimization_enabled")?
                    .as_bool()
                    .ok_or_eyre("invalid optimization_enabled")?
                {
                    1
                } else {
                    0
                },
                runs: response
                    .get("optimization_runs")
                    .ok_or_eyre("no optimization_runs")?
                    .as_u64()
                    .ok_or_eyre("invalid optimization_runs")?,
                constructor_arguments: alloy::primitives::Bytes::from_hex(
                    response
                        .get("constructor_args")
                        .ok_or_eyre("no constructor_args")?
                        .as_str()
                        .unwrap_or("0x"),
                )?,
                evm_version: response
                    .get("evm_version")
                    .ok_or_eyre("no evm_version")?
                    .as_str()
                    .ok_or_eyre("invalid evm_version")?
                    .to_string(),
                library: String::new(),
                license_type: String::new(),
                proxy: 0,
                implementation: None,
                swarm_source: String::new(),
            }],
        })
    }

    /// Fetches a contract's creation transaction hash and deployer address.
    pub async fn contract_creation_data(&self, address: Address) -> Result<ContractCreationData> {
        let url = format!("{}/api/v2/addresses/{}", self.base_url.trim_end_matches('/'), address);

        let response = self.client.get(&url).send().await?;
        let response = response.json::<Value>().await?;

        Ok(ContractCreationData {
            contract_address: address,
            contract_creator: response
                .get("creator_address_hash")
                .ok_or_eyre("no creator_address_hash")?
                .as_str()
                .ok_or_eyre("invalid creator_address_hash")?
                .parse()?,
            transaction_hash: response
                .get("creation_tx_hash")
                .ok_or_eyre("no creation_tx_hash")?
                .as_str()
                .ok_or_eyre("invalid creation_tx_hash")?
                .parse()?,
        })
    }
}
