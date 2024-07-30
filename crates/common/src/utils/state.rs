use std::collections::HashMap;

use alloy::primitives::U64;
use revm::primitives::U256;

/// State diff
#[derive(Debug, Clone, Default)]
pub struct PartialBlockStateDiff {
    /// Current balance after partial block execution. None if the balance was not touched.
    pub balance: Option<U256>,
    /// Current nonce after partial block execution. None if the nonce was not touched.
    pub nonce: Option<U64>,
    /// Current storage diff after partial block execution.
    pub storage: HashMap<U256, U256>,
}
