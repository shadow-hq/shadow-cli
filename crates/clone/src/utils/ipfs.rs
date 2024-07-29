use eyre::{eyre, Result};

/// Get the contents of a file from IPFS
pub(crate) async fn read_from_ipfs<T>(cid: &str, base_gateway_url: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned + Sized,
{
    let url = format!("{}/ipfs/{}", base_gateway_url.trim_end_matches('/'), cid);
    let response = reqwest::get(&url).await?;
    if response.status().is_success() {
        Ok(serde_json::from_str(&response.text().await?)?)
    } else {
        Err(eyre!("Failed to get file from IPFS: {}", response.text().await?))
    }
}
