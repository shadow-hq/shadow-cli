use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};

use alloy::{
    eips::{BlockId, BlockNumberOrTag},
    network::AnyNetwork,
    providers::RootProvider,
    transports::http::{Client, Http},
};
use eyre::{bail, Result};
use foundry_evm::backend::{BlockchainDb, BlockchainDbMeta, SharedBackend};
use parking_lot::RwLock;
use revm::{
    db::{AccountState, DbAccount},
    primitives::{AccountInfo, Address, BlockEnv, Bytecode, B256, U256},
    Database,
};
use tracing::trace;

use super::state::PartialBlockStateDiff;

/// An ephemeral, in-memory database implementation
/// which allows for overriding account bytecode.
#[derive(Debug, Clone)]
pub struct JsonRpcDatabase {
    /// Overrides
    overrides: HashMap<Address, Bytecode>,
    /// Partial-block state transitions to apply if touched by a transaction.
    partial_state: HashMap<Address, PartialBlockStateDiff>,
    /// Local database
    accounts: Arc<RwLock<HashMap<Address, DbAccount>>>,
    contracts: Arc<RwLock<HashMap<B256, Bytecode>>>,
    block_hashes: Arc<RwLock<HashMap<u64, B256>>>,
    /// Remote database
    remote_db: SharedBackend,
}

impl JsonRpcDatabase {
    /// Create a new [`JsonRpcDatabase`] instance.
    pub fn try_new(
        block_env: BlockEnv,
        provider: RootProvider<Http<Client>, AnyNetwork>,
        overrides: HashMap<Address, Bytecode>,
        partial_state: HashMap<Address, PartialBlockStateDiff>,
    ) -> Result<Self> {
        let remote_db = shared_backend(block_env, provider.clone())?;

        Ok(Self {
            remote_db,
            overrides,
            partial_state,
            accounts: Default::default(),
            contracts: Default::default(),
            block_hashes: Default::default(),
        })
    }

    /// Pop the partial state for the given address.
    pub fn partial_state(&mut self, address: Address) -> Option<PartialBlockStateDiff> {
        self.partial_state.remove(&address)
    }
}

impl Database for JsonRpcDatabase {
    type Error = eyre::Error;

    /// Get basic account information.
    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>> {
        // check for existing account
        if let Some(account) = self.accounts.read().get(&address) {
            return Ok(account.info());
        }

        // check for partial-block state transition
        let partial_state = self.partial_state(address);
        if partial_state.is_some() {
            trace!(address = format!("{:?}", address), "applying partial state transitions");
        }

        trace!(address = format!("{:?}", address), "missing account");

        // fetch the account from the remote database
        let account = foundry_evm::revm::DatabaseRef::basic_ref(&self.remote_db, address)?
            .map(|info| DbAccount {
                info: AccountInfo {
                    balance: partial_state
                        .as_ref()
                        .map(|s| s.balance)
                        .flatten()
                        .unwrap_or(info.balance),
                    nonce: partial_state
                        .as_ref()
                        .map(|s| s.nonce.map(|n| n.try_into().expect("U64 -> u64")))
                        .flatten()
                        .unwrap_or(info.nonce),
                    code_hash: info.code_hash,
                    code: self
                        .overrides
                        .get(&address)
                        .cloned()
                        .or_else(|| info.code.map(|code| Bytecode::new_raw(code.bytes()))),
                },
                storage: partial_state.as_ref().map(|s| s.storage.clone()).unwrap_or_default(),
                ..Default::default()
            })
            .unwrap_or_else(DbAccount::new_not_existing);

        // store the account in the local database
        self.accounts.write().insert(address, account.clone());

        Ok(account.info())
    }

    /// Get account code by its hash.
    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode> {
        // check for existing contract
        if let Some(contract) = self.contracts.read().get(&code_hash) {
            return Ok(contract.clone());
        }

        // check for existing override
        if let Some(contract) =
            self.overrides.values().find(|contract| contract.hash_slow() == code_hash)
        {
            return Ok(contract.clone());
        }

        trace!(code_hash = format!("{:?}", code_hash), "missing contract");

        let contract =
            foundry_evm::revm::DatabaseRef::code_by_hash_ref(&self.remote_db, code_hash)?;

        // store the contract in the local database
        self.contracts.write().insert(code_hash, Bytecode::new_raw(contract.bytes()));

        Ok(Bytecode::new_raw(contract.bytes()))
    }

    /// Get storage value of address at index.
    fn storage(&mut self, address: Address, index: U256) -> Result<U256> {
        // check for an existing account
        if let Some(account) = self.accounts.read().get(&address) {
            // check if the storage slot exists
            if let Some(slot) = account.storage.get(&index) {
                return Ok(U256::from(*slot));
            }

            // check if the account state has been cleared, or set to not existing
            if matches!(
                account.account_state,
                AccountState::StorageCleared | AccountState::NotExisting
            ) {
                return Ok(U256::ZERO);
            }
        }

        // check for partial-block state transition
        let partial_state = self.partial_state(address);
        if partial_state.is_some() {
            trace!(address = format!("{:?}", address), "applying partial state transitions");
        }

        // fetch the storage value from the remote database
        return match self.accounts.write().entry(address) {
            // if we don't have the account in our `accounts` map
            Entry::Vacant(entry) => {
                trace!(
                    address = format!("{:?}", address),
                    index = format!("{:?}", index),
                    "missing account, missing storage"
                );

                // fetch the account from the remote db
                let account_info =
                    foundry_evm::revm::DatabaseRef::basic_ref(&self.remote_db, address)?;
                if account_info.is_none() {
                    entry.insert(DbAccount::default());
                    return Ok(U256::ZERO);
                }

                // fetch the storage slot from the remote db
                let value =
                    foundry_evm::revm::DatabaseRef::storage_ref(&self.remote_db, address, index)?;
                let account_info = account_info.expect("impossible case: we should have exited");
                let mut account: DbAccount = DbAccount {
                    info: AccountInfo {
                        balance: partial_state
                            .as_ref()
                            .map(|s| s.balance)
                            .flatten()
                            .unwrap_or(account_info.balance),
                        nonce: partial_state
                            .as_ref()
                            .map(|s| s.nonce.map(|n| n.try_into().expect("U64 -> u64")))
                            .flatten()
                            .unwrap_or(account_info.nonce),
                        code_hash: account_info.code_hash,
                        code: account_info.code.map(|code| Bytecode::new_raw(code.bytes())),
                    },
                    storage: partial_state.as_ref().map(|s| s.storage.clone()).unwrap_or_default(),
                    account_state: AccountState::Touched,
                    ..Default::default()
                };
                account.storage.insert(index, value);

                // write the account
                entry.insert(account.clone());

                Ok(value)
            }
            // If we have the account in our `accounts` map
            Entry::Occupied(entry) => {
                trace!(
                    address = format!("{:?}", address),
                    index = format!("{:?}", index),
                    "cached account, missing storage"
                );

                // fetch the storage slot from the remote db
                let value =
                    foundry_evm::revm::DatabaseRef::storage_ref(&self.remote_db, address, index)?;

                // write the storage slot to the account
                entry.into_mut().storage.insert(index, value);

                return Ok(value);
            }
        };
    }

    /// Get block hash by block number.
    fn block_hash(&mut self, number: u64) -> Result<B256> {
        // check for existing block hash
        if let Some(hash) = self.block_hashes.read().get(&number) {
            return Ok(*hash);
        }

        trace!(number = number, "missing block hash");

        // fetch the block hash from the remote database
        let hash = foundry_evm::revm::DatabaseRef::block_hash_ref(&self.remote_db, number)?;

        // store the block hash in the local database
        self.block_hashes.write().insert(number, hash);

        Ok(hash)
    }
}

fn shared_backend(
    block_env: BlockEnv,
    provider: RootProvider<Http<Client>, AnyNetwork>,
) -> Result<SharedBackend> {
    // we need to mine the current block, so subtract 1
    if block_env.number == U256::ZERO || block_env.number == U256::from(1) {
        bail!("Cannot replay genesis block");
    }
    let block_number = block_env.number - U256::from(1);

    let mut cfg_env = foundry_evm::revm::primitives::CfgEnv::default();
    cfg_env.limit_contract_code_size = Some(usize::MAX);
    cfg_env.perf_analyse_created_bytecodes = foundry_evm::revm::primitives::AnalysisKind::Raw;
    cfg_env.disable_eip3607 = true;
    cfg_env.chain_id = 1u64;

    let meta = BlockchainDbMeta {
        cfg_env,
        block_env: foundry_evm::revm::primitives::BlockEnv {
            number: block_number,
            timestamp: block_env.timestamp,
            coinbase: block_env.coinbase,
            difficulty: block_env.difficulty,
            gas_limit: block_env.gas_limit,
            basefee: block_env.basefee,
            prevrandao: block_env.prevrandao,
            blob_excess_gas_and_price: block_env.blob_excess_gas_and_price.map(|a| {
                foundry_evm::revm::primitives::BlobExcessGasAndPrice {
                    excess_blob_gas: a.excess_blob_gas,
                    blob_gasprice: a.blob_gasprice,
                }
            }),
        },
        hosts: Default::default(),
    };

    Ok(SharedBackend::spawn_backend_thread(
        provider,
        BlockchainDb::new(meta, None),
        Some(BlockId::Number(BlockNumberOrTag::Number(block_number.try_into()?))),
    ))
}
