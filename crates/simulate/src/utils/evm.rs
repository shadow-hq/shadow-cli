use std::{collections::HashMap, path::PathBuf, str::FromStr};

use alloy::{
    primitives::{Address, Bytes, TxHash},
    rpc::types::trace::parity::{ChangedType, Delta, TraceResultsWithTransactionHash},
};
use eyre::{OptionExt, Result};
use hex::FromHex;
use revm::primitives::{AnalysisKind, BlobExcessGasAndPrice, BlockEnv, Bytecode, Env, TxEnv, U256};
use shadow_common::state::PartialBlockStateDiff;

/// Builds the EVM environment for the deployment
pub(crate) fn build_sim_env(
    from: Address,
    to: Option<Address>,
    original_value: U256,
    original_data: Bytes,
    block: BlockEnv,
) -> Box<Env> {
    let mut cfg_env = revm::primitives::CfgEnv::default();
    cfg_env.limit_contract_code_size = Some(usize::MAX);
    cfg_env.chain_id = 1u64;
    cfg_env.perf_analyse_created_bytecodes = AnalysisKind::Raw;
    Box::new(Env {
        cfg: cfg_env,
        tx: TxEnv {
            caller: from,
            gas_price: U256::from(0),
            gas_limit: u64::MAX,
            value: original_value,
            data: original_data,
            transact_to: revm::primitives::TxKind::Call(to.unwrap_or(from)),
            ..Default::default()
        },
        block: BlockEnv {
            blob_excess_gas_and_price: Some(BlobExcessGasAndPrice {
                excess_blob_gas: u64::MAX,
                blob_gasprice: 1,
            }),
            ..block
        },
    })
}

/// load bytecode overrides from artifact path
pub(crate) fn get_overrides(artifact_path: &PathBuf) -> Result<HashMap<Address, Bytecode>> {
    let mut overrides = HashMap::new();

    // walk the artifact_path recursively and collect all `.hex` files
    walkdir::WalkDir::new(artifact_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map(|ext| ext == "hex").unwrap_or(false))
        .try_for_each(|e| {
            // the address is the folder containing the hex file
            let address = e
                .path()
                .parent()
                .ok_or_eyre("invalid path")?
                .file_name()
                .ok_or_eyre("invalid path")?
                .to_str()
                .ok_or_eyre("invalid path")?;

            let address = Address::from_str(address)?;

            // read the hex file
            let bytecode = std::fs::read_to_string(e.path()).unwrap();
            let bytecode = Bytecode::LegacyRaw(Bytes::from_hex(bytecode)?);

            overrides.insert(address, bytecode);

            Ok::<_, eyre::Report>(())
        })?;

    Ok(overrides)
}

/// In order for us to only replay a single transaction in the block, we
/// can use the traces to build the block's state diff from `transaction_index` 0 to
/// `transaction_index` n and then replay the transaction at `transaction_index` n
pub(crate) fn build_state_diff(
    block_trace: Vec<TraceResultsWithTransactionHash>,
    transaction_hash: TxHash,
) -> Result<HashMap<Address, PartialBlockStateDiff>> {
    let mut accounts: HashMap<Address, PartialBlockStateDiff> = HashMap::new();

    for trace in block_trace {
        // once we reach the transaction we want to replay, we can stop
        if trace.transaction_hash == transaction_hash {
            break;
        }

        if let Some(state_diff) = trace.full_trace.state_diff {
            state_diff.0.iter().for_each(|(address, diff)| {
                let account = accounts.entry(*address).or_default();

                match diff.balance {
                    Delta::Added(balance) => account.balance = Some(balance),
                    Delta::Removed(_) => account.balance = None,
                    Delta::Changed(ChangedType { from: _, to }) => account.balance = Some(to),
                    _ => {}
                }

                match diff.nonce {
                    Delta::Added(nonce) => account.nonce = Some(nonce),
                    Delta::Removed(_) => account.nonce = None,
                    Delta::Changed(ChangedType { from: _, to }) => account.nonce = Some(to),
                    _ => {}
                }

                diff.storage.iter().for_each(|(key, value)| match value {
                    Delta::Added(value) => {
                        account
                            .storage
                            .insert(U256::from_be_slice(&key.0), U256::from_be_slice(&value.0));
                    }
                    Delta::Removed(_) => {
                        account.storage.remove(&U256::from_be_slice(&key.0));
                    }
                    Delta::Changed(ChangedType { from: _, to }) => {
                        account
                            .storage
                            .insert(U256::from_be_slice(&key.0), U256::from_be_slice(&to.0));
                    }
                    _ => {}
                });
            });
        }
    }

    Ok(accounts)
}
