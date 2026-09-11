//! Voluntary exits, ported from `ethstaker_deposit/utils/exit_transaction.py` and the
//! `exit-transaction-mnemonic` / `exit-transaction-keystore` commands.
//!
//! An exit is signed with the validator's *signing* key (`m/12381/3600/<i>/0/0`) under the
//! Capella fork version, so it can come either from the mnemonic or from a keystore file.

use std::path::{Path, PathBuf};

use bls::{PublicKeyBytes, SecretKey, SignatureBytes};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::chain::{ChainSetting, Network};
use crate::credential::Credential;
use crate::error::{Error, Result};
use crate::fs::{ensure_directory, unix_timestamp, write_sensitive_json};
use crate::keystore::decrypt_keystore_file;
use crate::mnemonic::{parse_mnemonic, Language};
use crate::spec::{
    compute_signing_root, compute_voluntary_exit_domain, SignedVoluntaryExit, VoluntaryExit,
};

/// JSON layout of a `signed_exit_transaction-*.json` file. Numbers are strings, the signature
/// carries `0x`, exactly like the deposit-cli writes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExitTransaction {
    pub message: ExitMessage,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExitMessage {
    pub epoch: String,
    pub validator_index: String,
}

impl From<&SignedVoluntaryExit> for ExitTransaction {
    fn from(signed: &SignedVoluntaryExit) -> Self {
        ExitTransaction {
            message: ExitMessage {
                epoch: signed.message.epoch.to_string(),
                validator_index: signed.message.validator_index.to_string(),
            },
            signature: format!("0x{}", hex::encode(signed.signature.serialize())),
        }
    }
}

/// `exit_transaction_generation` from the Python code.
pub fn sign_voluntary_exit(
    signing_key: &SecretKey,
    chain: &ChainSetting,
    validator_index: u64,
    epoch: u64,
) -> Result<SignedVoluntaryExit> {
    let genesis_validators_root = chain
        .genesis_validators_root
        .ok_or(Error::MissingGenesisValidatorsRoot)?;
    let message = VoluntaryExit {
        epoch,
        validator_index,
    };
    let domain = compute_voluntary_exit_domain(chain.exit_fork_version, genesis_validators_root);
    let signing_root = compute_signing_root(&message, domain);
    let signature = signing_key.sign(signing_root);
    Ok(SignedVoluntaryExit {
        message,
        signature: SignatureBytes::from(signature),
    })
}

/// `validate_signed_exit` from the Python code: the signature must be the given validator's
/// signature over the message under this chain's exit domain.
pub fn validate_signed_exit(
    transaction: &ExitTransaction,
    pubkey: PublicKeyBytes,
    chain: &ChainSetting,
) -> bool {
    let (Ok(epoch), Ok(validator_index)) = (
        transaction.message.epoch.parse::<u64>(),
        transaction.message.validator_index.parse::<u64>(),
    ) else {
        return false;
    };
    let Some(signature) =
        decode_hex(&transaction.signature).and_then(|b| SignatureBytes::deserialize(&b).ok())
    else {
        return false;
    };
    let Some(genesis_validators_root) = chain.genesis_validators_root else {
        return false;
    };
    let message = VoluntaryExit {
        epoch,
        validator_index,
    };
    let domain = compute_voluntary_exit_domain(chain.exit_fork_version, genesis_validators_root);
    let signing_root = compute_signing_root(&message, domain);
    match (pubkey.decompress(), signature.decompress()) {
        (Ok(pk), Ok(sig)) => sig.verify(&pk, signing_root),
        _ => false,
    }
}

/// Re-reads a `signed_exit_transaction-*.json` file and validates it.
pub fn verify_exit_transaction_file(
    path: &Path,
    pubkey: PublicKeyBytes,
    chain: &ChainSetting,
) -> Result<bool> {
    let file = std::fs::File::open(path).map_err(|e| Error::io("cannot open file", path, e))?;
    let transaction: ExitTransaction = serde_json::from_reader(file)?;
    Ok(validate_signed_exit(&transaction, pubkey, chain))
}

pub struct GenerateExitFromMnemonicRequest {
    pub folder: PathBuf,
    pub network: Network,
    pub mnemonic: Zeroizing<String>,
    pub mnemonic_password: Zeroizing<String>,
    pub mnemonic_language: Option<Language>,
    /// EIP-2334 index of the first validator; the i-th validator index below is signed by the
    /// key at `start_index + i`.
    pub start_index: u32,
    /// Beacon-chain validator indices, one per exit.
    pub validator_indices: Vec<u64>,
    /// Epoch from which the exit is valid; `0` means "as soon as possible".
    pub epoch: u64,
}

/// Writes one `signed_exit_transaction-<validator index>-<timestamp>.json` per validator and
/// re-reads each for verification. Returns the file paths in validator order.
pub fn generate_exit_transactions(req: &GenerateExitFromMnemonicRequest) -> Result<Vec<PathBuf>> {
    if req.validator_indices.is_empty() {
        return Err(Error::InvalidInput(
            "Please input at least one validator index.".to_string(),
        ));
    }
    let chain = req.network.setting();
    if chain.genesis_validators_root.is_none() {
        return Err(Error::MissingGenesisValidatorsRoot);
    }
    let count = u32::try_from(req.validator_indices.len())
        .map_err(|_| Error::InvalidInput("Too many validators.".to_string()))?;
    let end = req
        .start_index
        .checked_add(count)
        .ok_or_else(|| Error::InvalidInput("Validator index out of range.".to_string()))?;
    let mnemonic = parse_mnemonic(&req.mnemonic, req.mnemonic_language)?;
    let seed = mnemonic.seed(&req.mnemonic_password);
    let amount = chain.default_deposit_amount_gwei();
    let credentials: Vec<Credential> = (req.start_index..end)
        .into_par_iter()
        .map(|index| Credential::new(seed.as_ref(), index, amount, chain, None, false))
        .collect::<Result<_>>()?;

    let signed: Vec<SignedVoluntaryExit> = credentials
        .iter()
        .zip(&req.validator_indices)
        .map(|(credential, index)| credential.signed_voluntary_exit(*index, req.epoch))
        .collect::<Result<_>>()?;

    ensure_directory(&req.folder)?;
    let timestamp = unix_timestamp();
    let mut paths = Vec::with_capacity(signed.len());
    for (credential, exit) in credentials.iter().zip(&signed) {
        let path = write_exit_transaction(&req.folder, exit, timestamp)?;
        if !verify_exit_transaction_file(&path, credential.signing_pk(), chain)? {
            return Err(Error::ExitVerification);
        }
        paths.push(path);
    }
    Ok(paths)
}

pub struct GenerateExitFromKeystoreRequest {
    pub folder: PathBuf,
    pub network: Network,
    pub keystore_path: PathBuf,
    pub keystore_password: Zeroizing<String>,
    pub validator_index: u64,
    pub epoch: u64,
}

/// Signs an exit with the key in a keystore file, writes it and re-reads it for verification.
/// Returns the file path.
pub fn generate_exit_transaction_from_keystore(
    req: &GenerateExitFromKeystoreRequest,
) -> Result<PathBuf> {
    let chain = req.network.setting();
    if chain.genesis_validators_root.is_none() {
        return Err(Error::MissingGenesisValidatorsRoot);
    }
    let keypair = decrypt_keystore_file(&req.keystore_path, req.keystore_password.as_bytes())?;
    let signed = sign_voluntary_exit(&keypair.sk, chain, req.validator_index, req.epoch)?;

    ensure_directory(&req.folder)?;
    let path = write_exit_transaction(&req.folder, &signed, unix_timestamp())?;
    if !verify_exit_transaction_file(&path, keypair.pk.compress(), chain)? {
        return Err(Error::ExitVerification);
    }
    Ok(path)
}

fn write_exit_transaction(
    folder: &Path,
    signed: &SignedVoluntaryExit,
    timestamp: u64,
) -> Result<PathBuf> {
    let path = folder.join(format!(
        "signed_exit_transaction-{}-{timestamp}.json",
        signed.message.validator_index
    ));
    write_sensitive_json(&path, &ExitTransaction::from(signed))?;
    Ok(path)
}

fn decode_hex(s: &str) -> Option<Vec<u8>> {
    hex::decode(s.strip_prefix("0x").unwrap_or(s)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::MAINNET;
    use eth2_keystore::keypair_from_secret;

    fn keypair() -> bls::Keypair {
        keypair_from_secret(
            &hex::decode("000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f")
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn sign_and_validate_round_trip() {
        let kp = keypair();
        let signed = sign_voluntary_exit(&kp.sk, &MAINNET, 1, 1234).unwrap();
        let transaction = ExitTransaction::from(&signed);
        assert_eq!(transaction.message.epoch, "1234");
        assert_eq!(transaction.message.validator_index, "1");
        assert!(transaction.signature.starts_with("0x") && transaction.signature.len() == 194);
        assert!(validate_signed_exit(
            &transaction,
            kp.pk.compress(),
            &MAINNET
        ));

        // Any change to the message or the key breaks verification.
        let mut wrong_epoch = transaction.clone();
        wrong_epoch.message.epoch = "1235".to_string();
        assert!(!validate_signed_exit(
            &wrong_epoch,
            kp.pk.compress(),
            &MAINNET
        ));
        let other = keypair_from_secret(&[7u8; 32]).unwrap();
        assert!(!validate_signed_exit(
            &transaction,
            other.pk.compress(),
            &MAINNET
        ));
        assert!(!validate_signed_exit(
            &transaction,
            kp.pk.compress(),
            &crate::chain::HOODI
        ));

        let mut garbage = transaction.clone();
        garbage.signature = "0xzz".to_string();
        assert!(!validate_signed_exit(&garbage, kp.pk.compress(), &MAINNET));
        let mut not_a_number = transaction;
        not_a_number.message.validator_index = "one".to_string();
        assert!(!validate_signed_exit(
            &not_a_number,
            kp.pk.compress(),
            &MAINNET
        ));
    }

    #[test]
    fn ephemery_has_no_exit_domain() {
        assert!(matches!(
            sign_voluntary_exit(&keypair().sk, &crate::chain::EPHEMERY, 1, 0),
            Err(Error::MissingGenesisValidatorsRoot)
        ));
    }
}
