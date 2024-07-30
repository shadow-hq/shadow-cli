use std::{path::PathBuf, str::FromStr};

use alloy::{
    dyn_abi::EventExt,
    network::AnyNetwork,
    primitives::TxHash,
    providers::{ext::TraceApi, Provider, ProviderBuilder},
    rpc::types::trace::parity::TraceType,
    transports::http::reqwest::Url,
};
use eyre::{eyre, OptionExt, Result};
use revm::EvmBuilder;
use shadow_common::{
    db::JsonRpcDatabase, env::ReplayBlockEnv, forge::ensure_forge_installed,
    ShadowContractGroupInfo,
};
use tracing::{error, info, trace};

use crate::{
    event::{get_abis, try_get_event_abi, FullDecodedEvent, FullRawEvent, RawOrDecodedEvent},
    evm::{build_sim_env, build_state_diff, get_overrides},
    SimulateArgs,
};

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
                        log.topics().first().cloned().ok_or_eyre("cannot decode anonymous log")?;

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

            info!(
                "transaction succeeded:\n{}",
                logs.into_iter().map(|l| l.to_string()).collect::<Vec<String>>().join("\n")
            )
        }
        Err(e) => {
            error!("Failed to simulate transaction: {}", e);
        }
    };

    Ok(())
}
