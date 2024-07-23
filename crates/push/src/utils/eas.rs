use std::io::Write;

use alloy::{
    dyn_abi::DynSolValue,
    hex::FromHex,
    network::EthereumWallet,
    providers::{Provider, ProviderBuilder},
    signers::{
        ledger::LedgerSigner,
        local::{
            coins_bip39::English,
            yubihsm::{Connector, Credentials, UsbConfig},
            LocalSigner, MnemonicBuilder, PrivateKeySigner, YubiSigner,
        },
        trezor::TrezorSigner,
    },
    sol,
};
use eyre::{bail, eyre, OptionExt, Result};
use revm::primitives::{Address, Bytes, FixedBytes, U256};
use tracing::{debug, error, info, trace, warn};
use EAS::{AttestationRequest, AttestationRequestData};

use crate::{SignerType, SupportedChains};

// Codegen from ABI file to interact with EAS.
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    EAS,
    "abi/EAS.json"
);

/// Attempt to attest with EAS
pub(crate) async fn creator_attestation(
    ipfs_cid: &str,
    creator_address: &Address,
    signer_method: &SignerType,
    chain: &SupportedChains,
) -> Result<()> {
    warn!("EAS attestation from {:#020x} required to publish to https://logs.xyz", creator_address);
    let signer = match get_signer(signer_method, chain).await {
        Ok(signer) => signer,
        Err(e) => {
            warn!("failed to get signer: {}", e);
            return Ok(());
        }
    };
    if signer.default_signer().address() != *creator_address {
        error!(
            "signer address '{}' does not match creator address '{}'",
            signer.default_signer().address(),
            creator_address
        );
        bail!("signer address does not match creator address");
    }

    let provider = ProviderBuilder::new()
        .with_gas_estimation()
        .wallet(signer)
        .with_chain(chain.into())
        .on_http(chain.rpc_url());

    // Get the contract instance
    let eas = EAS::new(chain.eas_address(), provider.clone());
    let req = AttestationRequest {
        schema: chain.schema_uid().parse()?,
        data: AttestationRequestData {
            recipient: Address::ZERO,
            expirationTime: 0,
            revocable: true,
            refUID: FixedBytes::ZERO,
            data: Bytes::from_iter(DynSolValue::String(ipfs_cid.to_string()).abi_encode()),
            value: U256::ZERO,
        },
    };

    // build the attestation call
    let tx_nonce = provider.get_transaction_count(*creator_address).await?;
    let attestation_call =
        eas.attest(req).from(*creator_address).nonce(tx_nonce).chain_id(chain.chain_id());
    trace!("attestation call: {:#?}", attestation_call);

    // Prompt the user to confirm the attestation
    if prompt("You are about to sign an EAS attestation. Would you like to continue? (y/N): ")?
        .unwrap_or_else(|| "n".to_string())
        .as_str() !=
        "y"
    {
        warn!("user skipping EAS attestation");
        return Ok(());
    }

    // Send the attestation
    let attestation_tx_hash =
        provider.send_transaction(attestation_call.into_transaction_request()).await?;
    info!(
        "EAS attestation broadcast successfully: https://{}/tx/{}",
        chain.explorer_url(),
        attestation_tx_hash.tx_hash()
    );

    Ok(())
}

/// Get the signer for the given method
async fn get_signer(signer_method: &SignerType, chain: &SupportedChains) -> Result<EthereumWallet> {
    debug!("using --signer '{:?}'", signer_method);
    match signer_method {
        SignerType::PrivateKey => {
            let private_key = prompt("Enter your private key (or Enter to skip): ")?
                .ok_or_eyre("user skipping EAS attestation")?;
            let signer = PrivateKeySigner::from_bytes(
                &FixedBytes::<32>::from_hex(private_key)
                    .map_err(|e| eyre!("invalid private key: {}", e))?,
            )
            .map_err(|e| eyre!("failed to create signer: {}", e))?;
            Ok(EthereumWallet::from(signer))
        }
        SignerType::Mnemonic => {
            let mnemonic = prompt("Enter your mnemonic (or Enter to skip): ")?
                .ok_or_eyre("user skipping EAS attestation")?;
            let derivation_path = prompt("Enter your derivation path (m/44'/60'/0'/0/0): ")?
                .unwrap_or_else(|| "m/44'/60'/0'/0/0".to_string());
            let signer = MnemonicBuilder::<English>::default()
                .phrase(mnemonic)
                .derivation_path(derivation_path)?
                .build()
                .map_err(|e| eyre!("failed to create wallet: {}", e))?;
            Ok(EthereumWallet::from(signer))
        }
        SignerType::Ledger => {
            let hdpath = prompt("Enter your HDPath (0): ")?
                .unwrap_or_else(|| "0".to_string())
                .parse::<usize>()
                .map_err(|e| eyre!("invalid HDPath: {}", e))?;

            let signer = LedgerSigner::new(
                alloy::signers::ledger::HDPath::LedgerLive(hdpath),
                Some(chain.chain_id()),
            )
            .await?;
            Ok(EthereumWallet::from(signer))
        }
        SignerType::Trezor => {
            let hdpath = prompt("Enter your HDPath (0): ")?
                .unwrap_or_else(|| "0".to_string())
                .parse::<usize>()
                .map_err(|e| eyre!("invalid HDPath: {}", e))?;

            let signer = TrezorSigner::new(
                alloy::signers::trezor::HDPath::TrezorLive(hdpath),
                Some(chain.chain_id()),
            )
            .await?;
            Ok(EthereumWallet::from(signer))
        }
        SignerType::Yubikey => {
            let connector = Connector::usb(&UsbConfig::default());
            let signer = YubiSigner::connect(connector, Credentials::default(), 0);
            Ok(EthereumWallet::from(signer))
        }
        SignerType::Keystore => {
            let keystore = prompt("Enter the path to your keystore file (or Enter to skip): ")?
                .ok_or_eyre("user skipping EAS attestation")?;
            let password = prompt("Enter your keystore password: ")?
                .ok_or_eyre("user skipping EAS attestation")?;
            let signer = LocalSigner::decrypt_keystore(keystore, password)?;
            Ok(EthereumWallet::from(signer))
        }
    }
}

/// Prompt the user for input w/ pretty colors :D
fn prompt(text: &str) -> Result<Option<String>> {
    let mut input = String::new();
    const YELLOW_ANSI_CODE: &str = "\u{001b}[33m";
    const LIGHT_GRAY_ANSI_CODE: &str = "\u{001b}[90m";
    const RESET_ANSI_CODE: &str = "\u{001b}[0m";

    print!(
        "{LIGHT_GRAY_ANSI_CODE}{}  {YELLOW_ANSI_CODE}WARN{RESET_ANSI_CODE} {}",
        // include microsecond precision
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
        text,
    );

    std::io::stdout().flush().unwrap();
    std::io::stdin().read_line(&mut input)?;
    if !input.trim().is_empty() {
        return Ok(Some(input.trim().to_string()));
    }

    Ok(None)
}
