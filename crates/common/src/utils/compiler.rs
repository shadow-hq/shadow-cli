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
use tracing::error;

use crate::{ShadowContractInfo, ShadowContractSettings};

const DEPLOY_TX_GAS: u64 = 10000000000000000000;

fn construct_init_code(
    new_contract_bytecode: Bytes,
    original_constructor_arguments: &[u8],
) -> Bytes {
    let mut init_code = new_contract_bytecode.to_vec();
    init_code.extend_from_slice(original_constructor_arguments);

    Bytes::from(init_code)
}

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

// TODO: vyper support
/// Compile a contract using the original settings
pub fn compile(
    root: &PathBuf,
    settings: &ShadowContractSettings,
    metadata: &ShadowContractInfo,
) -> Result<Bytes> {
    // run `forge build` to compile the contract
    let build_artifact_dir = tempdir::TempDir::new("out")?;
    let output = std::process::Command::new("forge")
        .arg("build")
        .arg("--out")
        .arg(build_artifact_dir.path())
        .arg("--force")
        .arg("--no-cache")
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eyre::bail!("forge build failed: {}", stderr);
    }

    // list the files in the build artifact directory including nested directories
    let mut files = Vec::new();
    let walker = walkdir::WalkDir::new(build_artifact_dir.path());
    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                error!("failed to read entry: {}", err);
                continue;
            }
        };
        if entry.file_type().is_file() && entry.path().extension().unwrap_or_default() == "json" {
            files.push(entry.path().to_path_buf());
        }
    }

    // use strsim to find the closest match to the contract name with `.json` removed
    let contract_name = metadata.name.clone();
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
    let initcode = compiler_aritfacts
        .get("bytecode")
        .ok_or_else(|| eyre!("no bytecode found"))?
        .get("object")
        .ok_or_else(|| eyre!("no bytecode object found"))?
        .as_str()
        .ok_or_else(|| eyre!("bytecode is not a string"))?
        .to_owned();

    let compiler_output = CompilerOutput {
        abi: serde_json::from_value(compiler_aritfacts["abi"].clone())?,
        method_identifiers: compiler_aritfacts["methodIdentifiers"].clone(),
        bytecode: Bytes::from_hex(initcode)?,
    };

    // 2. simulate a deploy w/ the original settings and deployer
    let original_deployer = RevmAddress::from(metadata.contract_deployer);
    let original_init_code =
        construct_init_code(compiler_output.bytecode.clone(), &settings.constructor_arguments);

    let mut evm = EvmBuilder::default()
        .with_env({
            let mut cfg_env = revm::primitives::CfgEnv::default();
            cfg_env.limit_contract_code_size = Some(usize::MAX);
            cfg_env.chain_id = 1u64;
            cfg_env.perf_analyse_created_bytecodes = AnalysisKind::Raw;
            Box::new(Env {
                cfg: cfg_env.clone(),
                tx: TxEnv {
                    caller: original_deployer,
                    gas_price: U256::from(1),
                    gas_limit: DEPLOY_TX_GAS,
                    value: U256::ZERO,
                    data: original_init_code,
                    transact_to: TxKind::Create,
                    ..Default::default()
                },
                block: BlockEnv { number: U256::from(4470000), ..Default::default() },
                ..Default::default()
            })
        })
        .build();

    let result = evm.transact_preverified()?.result;
    let bytecode = result.into_output().ok_or_eyre("no bytecode")?;

    println!("Bytecode: {:?}", bytecode);

    Ok(bytecode)
}
