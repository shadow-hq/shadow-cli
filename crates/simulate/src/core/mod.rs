use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    path::PathBuf,
    str::FromStr,
};

use alloy::{
    dyn_abi::{DecodedEvent, EventExt},
    hex::FromHex,
    json_abi::{Event, JsonAbi},
    network::AnyNetwork,
    primitives::TxHash,
    providers::{ext::TraceApi, Provider, ProviderBuilder},
    rpc::types::trace::parity::{ChangedType, Delta, TraceResultsWithTransactionHash, TraceType},
    transports::http::reqwest::Url,
};
use eyre::{eyre, OptionExt, Result};
use revm::{
    primitives::{Address, AnalysisKind, BlockEnv, Bytecode, Bytes, Env, Log, TxEnv, B256, U256},
    EvmBuilder,
};
use shadow_common::{
    db::JsonRpcDatabase, env::ReplayBlockEnv, forge::ensure_forge_installed,
    state::PartialBlockStateDiff, ShadowContractGroupInfo,
};
use tracing::{error, info, trace};

use crate::SimulateArgs;

/// The `simulate` subcommand. Simulates a transaction with shadow overrides.
pub async fn simulate(args: SimulateArgs) -> Result<()> {
    // ensure forge is installed on the system
    ensure_forge_installed()?;

    // ensure args are valid
    args.validate().map_err(|e| eyre!("Invalid arguments: {}", e))?;
    let tx_hash: TxHash =
        args.transaction_hash.parse().map_err(|e| eyre!("Invalid transaction hash: {}", e))?;

    // root dir must be a shadow contract group
    let root_dir = PathBuf::from_str(&args.root)?;
    let mut group_info = ShadowContractGroupInfo::from_path(&root_dir).map_err(|e| {
        error!("This is not part of a shadow contract group.");
        eyre!("Failed to load shadow contract group: {}", e)
    })?;

    // validate that the group is ready for pinning
    info!("validating shadow contract group at {}", root_dir.display());
    group_info.validate().map_err(|e| eyre!("Failed to validate shadow contract group: {}", e))?;
    let artifact_path = group_info.prepare(&args.rpc_url).await?;

    // get a new provider
    let provider =
        ProviderBuilder::new().network::<AnyNetwork>().on_http(Url::parse(&args.rpc_url)?);

    info!("fetching transaction details for {}", tx_hash);
    let tx =
        provider.get_transaction_by_hash(tx_hash).await?.ok_or_eyre("transaction not found")?;
    let block_number = tx.block_number.ok_or_eyre("transaction not mined")?;

    info!("fetching block details for block {}", block_number);
    let block = provider
        .get_block_by_number(block_number.into(), true)
        .await?
        .ok_or_eyre("block not found")?;

    info!("fetching block trace for block {}", block_number);
    let block_trace = provider
        .trace_replay_block_transactions(
            block_number.into(),
            &[TraceType::StateDiff, TraceType::Trace],
        )
        .await?;

    let partial_block_state_diff = build_state_diff(block_trace, tx_hash)?;
    let overrides = get_overrides(&artifact_path)?;
    let abis = get_abis(&artifact_path)?;
    // let abis = Vec::new();

    trace!("contract overrides: {:?}", overrides.keys());
    info!("replaying transaction {}", tx_hash);

    let start_time = std::time::Instant::now();
    let block_env = ReplayBlockEnv::from(block);
    let db = JsonRpcDatabase::try_new(
        block_env.clone().into(),
        provider,
        overrides,
        partial_block_state_diff,
    )?;
    let env = build_sim_env(tx.from, tx.to, tx.value, tx.input.clone(), block_env.into());
    let mut evm = EvmBuilder::default().with_env(env).with_db(db).build();

    match evm.transact_preverified() {
        Ok(executed) => {
            if !executed.result.is_success() {
                error!("transaction failed: {:?}", executed.result);
                return Ok(());
            }
            info!("transaction executed in {:?}", start_time.elapsed());

            let logs = executed
                .result
                .logs()
                .iter()
                .enumerate()
                .map(|(transaction_log_index, log)| {
                    let event_selector =
                        log.topics().get(0).cloned().ok_or_eyre("cannot decode anonymous log")?;

                    let events = try_get_event_abi(&event_selector, &abis);

                    for event in events {
                        if let Ok(decoded) = event.decode_log(log, true) {
                            return Ok(RawOrDecodedEvent::Decoded(FullDecodedEvent {
                                inner: decoded,
                                event,
                                log: log.clone(),
                                transaction_log_index,
                            }));
                        }
                    }

                    Ok::<_, eyre::Report>(RawOrDecodedEvent::Raw(FullRawEvent {
                        log: log.clone(),
                        transaction_log_index,
                    }))
                })
                .collect::<Result<Vec<_>, _>>()?;

            for log in logs {
                println!("{}", log);
            }
        }
        Err(e) => {
            error!("Failed to simulate transaction: {}", e);
        }
    };

    Ok(())
}

/// Wrapper around a decoded event
#[derive(Debug, Clone)]
struct FullDecodedEvent {
    inner: DecodedEvent,
    event: Event,
    log: Log,
    transaction_log_index: usize,
}

/// Wrapper around a raw log
#[derive(Debug, Clone)]
struct FullRawEvent {
    log: Log,
    transaction_log_index: usize,
}

/// Wrapper enum for both raw and decoded events
#[derive(Debug, Clone)]
enum RawOrDecodedEvent {
    Raw(FullRawEvent),
    Decoded(FullDecodedEvent),
}

impl Display for RawOrDecodedEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RawOrDecodedEvent::Raw(log) => write!(
                f,
                r#"
Transaction Log Index : {}
Address               : {}
Event Selector        : {}
Event Signature       : N/A
Topic 1               : {}
Topic 2               : {}
Topic 3               : {}
Data                  : 0x{}
"#,
                log.transaction_log_index,
                log.log.address,
                log.log.topics()[0],
                log.log.topics().get(1).map(|t| t.to_string()).unwrap_or(String::from("N/A")),
                log.log.topics().get(2).map(|t| t.to_string()).unwrap_or(String::from("N/A")),
                log.log.topics().get(3).map(|t| t.to_string()).unwrap_or(String::from("N/A")),
                log.log
                    .data
                    .data
                    .to_vec()
                    .chunks(32)
                    .map(|chunk| hex::encode(chunk))
                    .collect::<Vec<_>>()
                    .join("\n                      :   ")
            ),
            RawOrDecodedEvent::Decoded(decoded) => {
                write!(
                    f,
                    r#"
Transaction Log Index : {}
Address               : {}
Event Selector        : {}
Event Signature       : {}
Decoded               :
                      : {}
"#,
                    decoded.transaction_log_index,
                    decoded.log.address,
                    decoded.log.topics()[0],
                    decoded.event.signature(),
                    decoded
                )
            }
        }
    }
}

impl Display for FullDecodedEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let decoded_str = self
            .inner
            .indexed
            .iter()
            .chain(self.inner.body.iter())
            .enumerate()
            .map(|(i, value)| {
                let name = self.event.inputs.get(i).map(|i| i.name.as_str()).unwrap_or("N/A");

                format!("{} {:?}", name, value)
            })
            .collect::<Vec<_>>()
            .join("\n                      : ");

        write!(f, "{}", decoded_str)
    }
}

/// Builds the EVM environment for the deployment
/// TODO: maybe trait this?
fn build_sim_env(
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
        block,
    })
}

fn get_overrides(artifact_path: &PathBuf) -> Result<HashMap<Address, Bytecode>> {
    let mut overrides = HashMap::new();

    // walk the artifact_path recursively and collect all `.hex` files
    walkdir::WalkDir::new(artifact_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map(|ext| ext == "hex").unwrap_or(false))
        .map(|e| {
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
        })
        .collect::<Result<_, _>>()?;

    Ok(overrides)
}

fn get_abis(artifact_path: &PathBuf) -> Result<Vec<JsonAbi>> {
    // walk the artifact_path recursively and collect all `.hex` files
    walkdir::WalkDir::new(artifact_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().file_name().map(|f| f == "abi.json").unwrap_or(false))
        .map(|e| {
            // parse the json file
            let json = std::fs::read_to_string(e.path()).unwrap();
            let abi = serde_json::from_str::<JsonAbi>(&json)?;
            Ok::<_, eyre::Report>(abi)
        })
        .collect::<Result<_, _>>()
}

/// In order for us to only replay a single transaction in the block, we
/// can use the traces to build the block's state diff from `transaction_index` 0 to
/// `transaction_index` n and then replay the transaction at `transaction_index` n
fn build_state_diff(
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
                let account =
                    accounts.entry(*address).or_insert_with(PartialBlockStateDiff::default);

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
                        account.storage.insert(
                            U256::try_from(*key).expect("impossible"),
                            U256::try_from(*value).expect("impossible"),
                        );
                    }
                    Delta::Removed(_) => {
                        account.storage.remove(&U256::try_from(*key).expect("impossible"));
                    }
                    Delta::Changed(ChangedType { from: _, to }) => {
                        account.storage.insert(
                            U256::try_from(*key).expect("impossible"),
                            U256::try_from(*to).expect("impossible"),
                        );
                    }
                    _ => {}
                });
            });
        }
    }

    Ok(accounts)
}

/// Try to get the event ABI(s) for the given event selector. Returns `None` if no event ABI is
/// found. Note: there may be multiple matching event signatures, so this function returns a Vec.
fn try_get_event_abi(selector: &B256, abis: &[JsonAbi]) -> Vec<Event> {
    abis.iter()
        .flat_map(|abi| abi.events.iter())
        .flat_map(|(_, events)| events.iter())
        .filter(|event| &event.selector() == selector)
        .cloned()
        .collect::<Vec<_>>()
}
