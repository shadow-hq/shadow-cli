use std::u64;

use alloy::rpc::types::Block;
use revm::primitives::{BlobExcessGasAndPrice, BlockEnv, SpecId, B256, U256};

/// A wrapper around [`BlockEnv`]
#[derive(Clone, Debug)]
pub struct ReplayBlockEnv {
    inner: BlockEnv,
}

impl From<ReplayBlockEnv> for BlockEnv {
    fn from(env: ReplayBlockEnv) -> Self {
        env.inner
    }
}

impl<T> From<Block<T>> for ReplayBlockEnv {
    fn from(block: Block<T>) -> Self {
        ReplayBlockEnv {
            inner: BlockEnv {
                number: U256::from(block.header.number.unwrap_or_default()),
                timestamp: U256::from(block.header.timestamp),
                gas_limit: U256::from(u64::MAX),
                difficulty: block.header.difficulty,
                prevrandao: Some(
                    block.header.mix_hash.map(|h| B256::from(h.0)).unwrap_or_default(),
                ),
                coinbase: block.header.miner,
                basefee: U256::from(block.header.base_fee_per_gas.unwrap_or(0)),
                blob_excess_gas_and_price: block.header.excess_blob_gas.map(|excess| {
                    BlobExcessGasAndPrice {
                        excess_blob_gas: excess.try_into().unwrap_or(0),
                        blob_gasprice: block.header.blob_fee().unwrap_or(0),
                    }
                }),
            },
        }
    }
}

/// Given block height, get the [`SpecId`] at that block height
pub fn get_eth_chain_spec(h: &u64) -> SpecId {
    // ranges taken from https://github.com/ethereum/execution-specs
    match h {
        0..=199999 => SpecId::FRONTIER,
        200000..=1149999 => SpecId::HOMESTEAD,
        1150000..=1919999 => SpecId::DAO_FORK,
        1920000..=2462999 => SpecId::TANGERINE,
        2463000..=2674999 => SpecId::SPURIOUS_DRAGON,
        2675000..=4369999 => SpecId::BYZANTIUM,
        4370000..=7279999 => SpecId::PETERSBURG, // CONSTANTINOPLE
        7280000..=9199999 => SpecId::ISTANBUL,
        9200000..=12243999 => SpecId::MUIR_GLACIER,
        12244000..=12964999 => SpecId::BERLIN,
        12965000..=13772999 => SpecId::LONDON,
        13773000..=15049999 => SpecId::ARROW_GLACIER,
        15050000..=15537393 => SpecId::GRAY_GLACIER,
        15537394..=17034869 => SpecId::MERGE, // PARIS
        17034870..=19426586 => SpecId::SHANGHAI,
        19426587..=u64::MAX => SpecId::CANCUN, // LATEST
    }
}
