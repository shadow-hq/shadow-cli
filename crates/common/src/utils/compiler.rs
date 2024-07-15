use alloy::primitives::bytes::Bytes;
use alloy_json_abi::JsonAbi;
use eyre::{eyre, Result};
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

// TODO: vyper support
/// Compile a contract using the original settings
pub fn compile(
    root: &PathBuf,
    settings: &ShadowContractSettings,
    metadata: &ShadowContractInfo,
) -> Result<()> {
    let output_dir = root.join("out");

    // run `forge build` to compile the contract
    let build_artifact_dir = tempdir::TempDir::new("out")?;
    let output = std::process::Command::new("forge")
        .arg("build")
        .arg("--out")
        .arg(build_artifact_dir.path())
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
        let entry = entry?;
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

    println!("{:#?}", compiler_aritfacts);

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
        bytecode: Bytes::try_from(initcode)?,
    };

    println!("{:#?}", compiler_output);

    Ok(())
}
