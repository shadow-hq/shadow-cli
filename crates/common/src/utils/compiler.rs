use crate::{
    db::JsonRpcDatabase,
    env::{get_eth_chain_spec, ReplayBlockEnv},
    ShadowContractInfo, ShadowContractSettings,
};
use alloy::{
    hex::FromHex,
    network::AnyNetwork,
    providers::{Provider, ProviderBuilder},
    transports::http::reqwest::Url,
};
use alloy_json_abi::JsonAbi;
use eyre::{eyre, OptionExt, Result};
use revm::{
    primitives::{Address as RevmAddress, AnalysisKind, Bytes, Env, TxEnv, TxKind, U256},
    EvmBuilder,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tracing::{error, info};

/// Compiler Output
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompilerOutput {
    /// The abi of the compiled contract
    pub abi: JsonAbi,
    /// Method identifiers of the compiled contract
    #[serde(rename = "methodIdentifiers")]
    pub method_identifiers: Value,
    /// The bytecode of the compiled contract
    pub bytecode: Bytes,
}

/// Compile a contract using the original settings.
/// TODO @jon-becker: Ensure vyper is supported
pub async fn compile(
    rpc_url: &str,
    root: &PathBuf,
    settings: &ShadowContractSettings,
    metadata: &ShadowContractInfo,
) -> Result<CompilerOutput> {
    // create the artifact directory
    let build_artifact_dir = root.join("out");
    std::fs::create_dir_all(&build_artifact_dir)?;

    // compile via forge
    compile_contract(root).map_err(|e| eyre!("failed to compile: {}", e))?;

    // find the contract artifact in the build directory
    let (contract_artifact, artifact_path) =
        find_contract_artifact(&build_artifact_dir, &metadata.name)
            .map_err(|e| eyre!("contract artifact not found: {}", e))?;
    let shadow_artifact_path = artifact_path.with_file_name(format!(
        "{}.shadow.json",
        artifact_path.file_stem().unwrap().to_str().unwrap()
    ));

    // simulate the contract deployment w/ the original settings and deployer
    let provider = ProviderBuilder::new().network::<AnyNetwork>().on_http(Url::parse(rpc_url)?);

    info!("fetching transaction details for {}", metadata.deployment_transaction_hash);
    let tx = provider
        .get_transaction_by_hash(metadata.deployment_transaction_hash)
        .await?
        .ok_or_eyre("transaction not found")?;
    let block_number = tx.block_number.ok_or_eyre("transaction not mined")?;

    info!("fetching block details for block {}", block_number);
    let block = provider
        .get_block_by_number(block_number.into(), true)
        .await?
        .ok_or_eyre("block not found")?;
    let replay_block_env = ReplayBlockEnv::from(block);
    let db = JsonRpcDatabase::try_new(
        replay_block_env.clone().into(),
        provider,
        HashMap::new(),
        HashMap::new(),
    )?;

    info!("constructing runtime bytecode");
    let initcode = construct_init_code(&contract_artifact, &settings.constructor_arguments)
        .map_err(|e| eyre!("failed to construct init code: {}", e))?;
    let deployment_env =
        build_deployment_env(metadata.contract_deployer, initcode, replay_block_env);

    // execute the deployment transaction
    let mut evm = EvmBuilder::default()
        .with_db(db)
        .with_spec_id(get_eth_chain_spec(&block_number))
        .with_env(deployment_env)
        .build();
    let output =
        evm.transact_preverified().map_err(|e| eyre!("failed to deploy contract: {}", e))?;

    let compiler_output = CompilerOutput {
        abi: serde_json::from_value(contract_artifact["abi"].clone())?,
        method_identifiers: contract_artifact["methodIdentifiers"].clone(),
        bytecode: output.result.into_output().ok_or_eyre("no bytecode")?,
    };

    // serialize and write the shadow artifact
    std::fs::write(shadow_artifact_path, serde_json::to_string_pretty(&compiler_output)?)?;

    Ok(compiler_output)
}

/// Construct the init code for the contract by concatenating the new contract
/// bytecode with the original constructor arguments
fn construct_init_code(
    contract_artifact: &Value,
    original_constructor_arguments: &[u8],
) -> Result<Bytes> {
    let new_contract_bytecode = Bytes::from_hex(
        contract_artifact
            .get("bytecode")
            .ok_or_else(|| eyre!("no bytecode found"))?
            .get("object")
            .ok_or_else(|| eyre!("no bytecode object found"))?
            .as_str()
            .ok_or_else(|| eyre!("bytecode is not a string"))?,
    )?;

    let mut init_code = new_contract_bytecode.to_vec();
    init_code.extend_from_slice(original_constructor_arguments);

    Ok(Bytes::from(init_code))
}

/// Compiles all contracts at the given path by invoking the forge build command
fn compile_contract(root: &PathBuf) -> Result<()> {
    let output = std::process::Command::new("forge")
        .arg("build")
        .arg("--force")
        .arg("--no-cache")
        .current_dir(root)
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("{}", stderr);

        eyre::bail!("build failed");
    }

    Ok(())
}

/// Find the contract artifact in the build artifact directory
fn find_contract_artifact(
    build_artifact_dir: &Path,
    contract_name: &str,
) -> Result<(Value, PathBuf)> {
    // find all artifacts in the build artifact directory
    let mut files = Vec::new();
    let walker = walkdir::WalkDir::new(build_artifact_dir);
    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        if entry.file_type().is_file() && entry.path().extension().unwrap_or_default() == "json" {
            files.push(entry.path().to_path_buf());
        }
    }

    // use strsim to find the closest match to the contract name with `.json` removed
    let mut closest_match = None;
    let mut closest_distance = usize::MAX;
    for file in &files {
        let file_stem = file.file_stem().unwrap().to_string_lossy();
        let distance = strsim::levenshtein(contract_name, &file_stem);
        if distance < closest_distance {
            closest_distance = distance;
            closest_match = Some(file);
        }
    }

    // if no match is found, return an error
    let closest_match = closest_match.ok_or_else(|| eyre!("no contract artifact found"))?;
    let compiler_aritfacts: Value = serde_json::from_reader(std::fs::File::open(closest_match)?)?;

    Ok((compiler_aritfacts, closest_match.to_owned()))
}

/// Builds the EVM environment for the deployment
fn build_deployment_env(
    original_deployer: RevmAddress,
    initcode: Bytes,
    replay_block_env: ReplayBlockEnv,
) -> Box<Env> {
    let mut cfg_env = revm::primitives::CfgEnv::default();
    cfg_env.limit_contract_code_size = Some(usize::MAX);
    cfg_env.chain_id = 1u64;
    cfg_env.perf_analyse_created_bytecodes = AnalysisKind::Raw;
    Box::new(Env {
        cfg: cfg_env,
        tx: TxEnv {
            caller: original_deployer,
            gas_price: U256::from(0),
            gas_limit: u64::MAX,
            value: U256::ZERO,
            data: initcode,
            transact_to: TxKind::Create,
            ..Default::default()
        },
        block: replay_block_env.into(),
    })
}
