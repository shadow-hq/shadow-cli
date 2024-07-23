use std::path::PathBuf;

use eyre::{eyre, Result};
use pinata_sdk::{PinByFile, PinataApi};

/// Result of pinning a contract group
#[derive(Debug, Clone)]
pub(crate) struct PinResult {
    /// The CID of the pinned contract group
    pub(crate) cid: String,
    /// The IPFS URL of the pinned contract group
    pub(crate) ipfs_url: String,
}

/// Pins the provided
pub(crate) async fn pin_shadow_contract_group(
    path: &PathBuf,
    api_key: &str,
    secret_api_key: &str,
    base_gateway_url: &str,
) -> Result<PinResult> {
    let api = PinataApi::new(api_key, secret_api_key)
        .map_err(|e| eyre!("Failed to create Pinata API client: {}", e))?;
    let result = api
        .pin_file(PinByFile::new(format!("{}/", path.to_string_lossy().to_string())))
        .await
        .map_err(|e| eyre!("Failed to pin file: {}", e))?;

    Ok(PinResult {
        cid: result.ipfs_hash.clone(),
        ipfs_url: format!("{}/{}/", base_gateway_url.trim_end_matches('/'), result.ipfs_hash),
    })
}
