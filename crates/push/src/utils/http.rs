use eyre::{eyre, Result};

/// Calls out to `https://logs.xyz/api/pin` with the IPFS CID of the shadow contract group.
///
/// This will pin your group to a permanent IPFS node, as well as enable group search on logs.xyz.
pub async fn pin_to_logs_xyz_ipfs_node(ipfs_cid: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://logs.xyz/api/pin")
        .json(&serde_json::json!({ "ipfs_cid": ipfs_cid }))
        .send()
        .await?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(eyre!("Failed to pin shadow contract group to logs.xyz: {}", response.text().await?))
    }
}
