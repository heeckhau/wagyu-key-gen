//! Validator key generation: keystores plus deposit data, ported from
//! `stakingdeposit_proxy.generate_keys` and `ethstaker_deposit/utils/validation.py::validate_deposit`.

use std::path::{Path, PathBuf};

use bls::{PublicKeyBytes, SignatureBytes};
use rayon::prelude::*;
use tree_hash::TreeHash;
use zeroize::Zeroizing;

use crate::chain::{ChainSetting, Network, MAX_DEPOSIT_AMOUNT_GWEI};
use crate::credential::{Credential, DepositDatum};
use crate::error::{Error, Result};
use crate::fs::{ensure_directory, unix_timestamp, write_sensitive_json};
use crate::keystore::{keystore_file_name, read_keystore_file, write_keystore_file, KdfChoice};
use crate::mnemonic::{parse_mnemonic, Language};
use crate::spec::{compute_deposit_domain, compute_signing_root, DepositData, DepositMessage};
use crate::validation::{
    parse_optional_address, validate_password, BLS_WITHDRAWAL_PREFIX,
    COMPOUNDING_WITHDRAWAL_PREFIX, EXECUTION_ADDRESS_WITHDRAWAL_PREFIX,
};

/// Upper bound on concurrent scrypt operations. Each one needs about 256 MiB of RAM.
pub const MAX_KEYSTORE_THREADS: usize = 4;

pub struct GenerateKeysRequest {
    pub mnemonic: Zeroizing<String>,
    /// Optional BIP-39 passphrase ("25th word"). Wagyu always passes the empty string.
    pub mnemonic_password: Zeroizing<String>,
    /// Restrict mnemonic validation to one language; `None` auto-detects.
    pub mnemonic_language: Option<Language>,
    pub start_index: u32,
    pub count: u32,
    /// Deposit per validator in gwei, already multiplied for Gnosis chains
    /// (see [`crate::validation::deposit_amount_to_gwei`]).
    pub amount_gwei: u64,
    pub network: Network,
    pub keystore_password: Zeroizing<String>,
    /// Withdrawal address; `None` or empty gives BLS (`0x00`) credentials.
    pub withdrawal_address: Option<String>,
    pub compounding: bool,
    pub folder: PathBuf,
    pub kdf: KdfChoice,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct GenerateKeysOutput {
    pub keystore_files: Vec<PathBuf>,
    pub deposit_data_file: PathBuf,
    /// Hex signing public keys, in index order.
    pub pubkeys: Vec<String>,
}

/// Generates `count` validators from `start_index`, writes one keystore per validator and one
/// `deposit_data-<timestamp>.json`, then re-reads and verifies every file.
pub fn generate_keys(req: &GenerateKeysRequest) -> Result<GenerateKeysOutput> {
    validate_password(&req.keystore_password)?;
    if req.count == 0 {
        return Err(Error::InvalidInput(
            "The number of validators must be at least 1.".to_string(),
        ));
    }
    let end_index = req
        .start_index
        .checked_add(req.count)
        .ok_or_else(|| Error::InvalidInput("Validator index out of range.".to_string()))?;
    let withdrawal_address = parse_optional_address(req.withdrawal_address.as_deref())?;
    if req.compounding && withdrawal_address.is_none() {
        return Err(Error::MissingAddress);
    }
    let chain = req.network.setting();
    let mnemonic = parse_mnemonic(&req.mnemonic, req.mnemonic_language)?;
    let seed = mnemonic.seed(&req.mnemonic_password);
    let password = req.keystore_password.as_bytes();

    let credentials: Vec<Credential> = (req.start_index..end_index)
        .into_par_iter()
        .map(|index| {
            Credential::new(
                seed.as_ref(),
                index,
                req.amount_gwei,
                chain,
                withdrawal_address,
                req.compounding,
            )
        })
        .collect::<Result<_>>()?;
    // Fail on a bad amount before any file is written.
    for credential in &credentials {
        credential.deposit_message()?;
    }

    ensure_directory(&req.folder)?;
    let timestamp = unix_timestamp();
    let pool = keystore_pool()?;

    let keystore_files: Vec<PathBuf> = pool.install(|| {
        credentials
            .par_iter()
            .map(|credential| {
                let keystore = credential.signing_keystore(password, req.kdf)?;
                let path = req.folder.join(keystore_file_name(
                    &credential.signing_key_path(),
                    timestamp,
                ));
                write_keystore_file(&path, &keystore)?;
                Ok(path)
            })
            .collect::<Result<_>>()
    })?;

    let deposit_data: Vec<DepositDatum> = credentials
        .iter()
        .map(Credential::deposit_datum)
        .collect::<Result<_>>()?;
    let deposit_data_file = req.folder.join(format!("deposit_data-{timestamp}.json"));
    write_sensitive_json(&deposit_data_file, &deposit_data)?;

    let keystores_ok = pool.install(|| {
        credentials
            .par_iter()
            .zip(keystore_files.par_iter())
            .map(|(credential, path)| {
                credential.verify_keystore(&read_keystore_file(path)?, password)
            })
            .collect::<Result<Vec<bool>>>()
    })?;
    if !keystores_ok.iter().all(|ok| *ok) {
        return Err(Error::KeystoreVerification);
    }

    if !verify_deposit_data_file(&deposit_data_file, &credentials, chain)? {
        return Err(Error::DepositVerification);
    }

    Ok(GenerateKeysOutput {
        keystore_files,
        deposit_data_file,
        pubkeys: deposit_data.into_iter().map(|d| d.pubkey).collect(),
    })
}

/// Re-reads a `deposit_data-*.json` file and validates every entry against `credentials`.
pub fn verify_deposit_data_file(
    path: &Path,
    credentials: &[Credential],
    chain: &ChainSetting,
) -> Result<bool> {
    let file = std::fs::File::open(path).map_err(|e| Error::io("cannot open file", path, e))?;
    let deposits: Vec<DepositDatum> = serde_json::from_reader(file)?;
    if deposits.len() != credentials.len() {
        return Ok(false);
    }
    Ok(deposits
        .iter()
        .zip(credentials)
        .all(|(deposit, credential)| validate_deposit(deposit, chain, Some(credential))))
}

/// Checks one deposit against the staking deposit rules and, if given, against the credential it
/// should have been produced from (`validate_deposit` in the Python code).
pub fn validate_deposit(
    deposit: &DepositDatum,
    chain: &ChainSetting,
    credential: Option<&Credential>,
) -> bool {
    let Some(pubkey) = hex::decode(&deposit.pubkey)
        .ok()
        .and_then(|b| PublicKeyBytes::deserialize(&b).ok())
    else {
        return false;
    };
    let Ok(withdrawal_credentials) = hex::decode(&deposit.withdrawal_credentials) else {
        return false;
    };
    let Some(signature) = hex::decode(&deposit.signature)
        .ok()
        .and_then(|b| SignatureBytes::deserialize(&b).ok())
    else {
        return false;
    };
    let Ok(deposit_data_root) = hex::decode(&deposit.deposit_data_root) else {
        return false;
    };
    let Ok(deposit_message_root) = hex::decode(&deposit.deposit_message_root) else {
        return false;
    };
    let Ok(fork_version) = hex::decode(&deposit.fork_version).map(<[u8; 4]>::try_from) else {
        return false;
    };
    let Ok(fork_version) = fork_version else {
        return false;
    };

    // Pubkey.
    if let Some(credential) = credential {
        if pubkey != credential.signing_pk() {
            return false;
        }
    }

    // Withdrawal credentials.
    if withdrawal_credentials.len() != 32 {
        return false;
    }
    match withdrawal_credentials[0] {
        BLS_WITHDRAWAL_PREFIX => {
            if let Some(credential) = credential {
                if credential.withdrawal_prefix() != BLS_WITHDRAWAL_PREFIX {
                    return false;
                }
                if withdrawal_credentials[1..]
                    != credential.bls_withdrawal_credentials().as_slice()[1..]
                {
                    return false;
                }
            }
        }
        EXECUTION_ADDRESS_WITHDRAWAL_PREFIX | COMPOUNDING_WITHDRAWAL_PREFIX => {
            if withdrawal_credentials[1..12].iter().any(|b| *b != 0) {
                return false;
            }
            if let Some(credential) = credential {
                if credential.withdrawal_prefix() != withdrawal_credentials[0] {
                    return false;
                }
                match credential.withdrawal_address() {
                    Some(address) if address.as_slice() == &withdrawal_credentials[12..] => {}
                    _ => return false,
                }
            }
        }
        _ => return false,
    }

    // Amount.
    if deposit.amount < chain.min_deposit_message_amount_gwei()
        || deposit.amount > MAX_DEPOSIT_AMOUNT_GWEI
    {
        return false;
    }

    // Signature.
    let message = DepositMessage {
        pubkey,
        withdrawal_credentials: bls::Hash256::from_slice(&withdrawal_credentials),
        amount: deposit.amount,
    };
    if message.tree_hash_root().as_slice() != deposit_message_root.as_slice() {
        return false;
    }
    let domain = compute_deposit_domain(fork_version);
    let signing_root = compute_signing_root(&message, domain);
    let (Ok(pk), Ok(sig)) = (pubkey.decompress(), signature.decompress()) else {
        return false;
    };
    if !sig.verify(&pk, signing_root) {
        return false;
    }

    // Deposit data root.
    let signed = DepositData {
        pubkey,
        withdrawal_credentials: message.withdrawal_credentials,
        amount: deposit.amount,
        signature,
    };
    signed.tree_hash_root().as_slice() == deposit_data_root.as_slice()
}

fn keystore_pool() -> Result<rayon::ThreadPool> {
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .clamp(1, MAX_KEYSTORE_THREADS);
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .map_err(|e| Error::InvalidInput(format!("cannot start worker threads: {e}")))
}
