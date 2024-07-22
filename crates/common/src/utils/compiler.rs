use alloy::hex::FromHex;
use alloy_json_abi::JsonAbi;
use eyre::{eyre, OptionExt, Result};
use revm::{
    primitives::{Address as RevmAddress, AnalysisKind, BlockEnv, Bytes, Env, TxEnv, TxKind, U256},
    EvmBuilder,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

use crate::{ShadowContractInfo, ShadowContractSettings};

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
pub fn compile(
    root: &PathBuf,
    settings: &ShadowContractSettings,
    metadata: &ShadowContractInfo,
) -> Result<CompilerOutput> {
    // create the artifact directory
    let build_artifact_dir = root.join("out");
    std::fs::create_dir_all(&build_artifact_dir)?;

    // compile via forge
    let _ = compile_contract(root).map_err(|e| eyre!("failed to compile contract {}", e))?;

    // find the contract artifact in the build directory
    let contract_artifact = find_contract_artifact(&build_artifact_dir, &metadata.name)
        .map_err(|e| eyre!("contract artifact not found: {}", e))?;

    // simulate the contract deployment w/ the original settings and deployer
    // TODO @jon-becker: we might need an anvil fork running if the constructor calls out to other
    // contracts
    let original_deployer = RevmAddress::from(metadata.contract_deployer);
    let initcode = construct_init_code(&contract_artifact, &settings.constructor_arguments)
        .map_err(|e| eyre!("failed to construct init code: {}", e))?;
    let deployment_env = build_deployment_env(original_deployer, initcode);

    // execute the deployment transaction
    let mut evm = EvmBuilder::default().with_env(deployment_env).build();
    let result =
        evm.transact_preverified().map_err(|e| eyre!("failed to deploy contract: {}", e))?.result;

    Ok(CompilerOutput {
        abi: serde_json::from_value(contract_artifact["abi"].clone())?,
        method_identifiers: contract_artifact["methodIdentifiers"].clone(),
        bytecode: result.into_output().ok_or_eyre("no bytecode")?,
    })
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
            .ok_or_else(|| eyre!("bytecode is not a string"))?
            .to_owned(),
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
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eyre::bail!("forge build failed: {}", stderr);
    }

    Ok(())
}

/// Find the contract artifact in the build artifact directory
fn find_contract_artifact(build_artifact_dir: &PathBuf, contract_name: &str) -> Result<Value> {
    // find all artifacts in the build artifact directory
    let mut files = Vec::new();
    let walker = walkdir::WalkDir::new(build_artifact_dir.as_path());
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
        let distance = strsim::levenshtein(&contract_name, &file_stem);
        if distance < closest_distance {
            closest_distance = distance;
            closest_match = Some(file);
        }
    }

    // if no match is found, return an error
    let closest_match = closest_match.ok_or_else(|| eyre!("no contract artifact found"))?;
    let compiler_aritfacts: Value = serde_json::from_reader(std::fs::File::open(closest_match)?)?;

    Ok(compiler_aritfacts)
}

/// Builds the EVM environment for the deployment
fn build_deployment_env(original_deployer: RevmAddress, initcode: Bytes) -> Box<Env> {
    let mut cfg_env = revm::primitives::CfgEnv::default();
    cfg_env.limit_contract_code_size = Some(usize::MAX);
    cfg_env.chain_id = 1u64;
    cfg_env.perf_analyse_created_bytecodes = AnalysisKind::Raw;
    Box::new(Env {
        cfg: cfg_env.clone(),
        tx: TxEnv {
            caller: original_deployer,
            gas_price: U256::from(1),
            gas_limit: u64::MAX,
            value: U256::ZERO,
            data: initcode,
            transact_to: TxKind::Create,
            ..Default::default()
        },
        block: BlockEnv { number: U256::from(4470000), ..Default::default() },
        ..Default::default()
    })
}
