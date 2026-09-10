//! BLS-to-execution changes, ported from `stakingdeposit_proxy.generate_bls_to_execution_change`,
//! `validate_bls_credentials` and `ethstaker_deposit/utils/validation.py`.

use std::path::{Path, PathBuf};

use alloy_primitives::Address;
use bls::{Hash256, PublicKeyBytes, SignatureBytes};
use rayon::prelude::*;
use zeroize::Zeroizing;

use crate::chain::{ChainSetting, Network};
use crate::credential::{BtecEntry, Credential};
use crate::error::{Error, Result};
use crate::fs::{ensure_directory, unix_timestamp, write_sensitive_json};
use crate::mnemonic::{parse_mnemonic, Language};
use crate::spec::{
    compute_bls_to_execution_change_domain, compute_signing_root, BlsToExecutionChange,
};
use crate::validation::parse_address;

pub struct ValidateBlsCredentialsRequest {
    pub network: Network,
    pub mnemonic: Zeroizing<String>,
    pub mnemonic_password: Zeroizing<String>,
    pub mnemonic_language: Option<Language>,
    pub start_index: u32,
    pub bls_withdrawal_credentials: Vec<Hash256>,
}

/// Checks that every given `0x00` credential is the one derived from the mnemonic at
/// `start_index + i`.
pub fn validate_bls_credentials(req: &ValidateBlsCredentialsRequest) -> Result<()> {
    let chain = req.network.setting();
    let mnemonic = parse_mnemonic(&req.mnemonic, req.mnemonic_language)?;
    let seed = mnemonic.seed(&req.mnemonic_password);
    let credentials = derive_credentials(
        seed.as_ref(),
        req.start_index,
        req.bls_withdrawal_credentials.len(),
        chain,
        None,
    )?;
    check_credentials_match(&credentials, &req.bls_withdrawal_credentials)
}

pub struct GenerateBtecRequest {
    pub folder: PathBuf,
    pub network: Network,
    pub mnemonic: Zeroizing<String>,
    pub mnemonic_password: Zeroizing<String>,
    pub mnemonic_language: Option<Language>,
    pub start_index: u32,
    /// Beacon-chain validator indices, one per credential.
    pub validator_indices: Vec<u64>,
    pub bls_withdrawal_credentials: Vec<Hash256>,
    pub withdrawal_address: String,
}

/// Writes `bls_to_execution_change-<timestamp>.json` with one signed change per validator and
/// re-reads it for verification. Returns the file path.
pub fn generate_bls_to_execution_change(req: &GenerateBtecRequest) -> Result<PathBuf> {
    if req.validator_indices.len() != req.bls_withdrawal_credentials.len() {
        return Err(Error::IndicesCredentialsMismatch);
    }
    if req.validator_indices.is_empty() {
        return Err(Error::InvalidInput(
            "Please input at least one validator index.".to_string(),
        ));
    }
    let withdrawal_address = parse_address(&req.withdrawal_address)?;
    let chain = req.network.setting();
    if chain.genesis_validators_root.is_none() {
        return Err(Error::MissingGenesisValidatorsRoot);
    }
    let mnemonic = parse_mnemonic(&req.mnemonic, req.mnemonic_language)?;
    let seed = mnemonic.seed(&req.mnemonic_password);
    let credentials = derive_credentials(
        seed.as_ref(),
        req.start_index,
        req.validator_indices.len(),
        chain,
        Some(withdrawal_address),
    )?;
    check_credentials_match(&credentials, &req.bls_withdrawal_credentials)?;

    let entries: Vec<BtecEntry> = credentials
        .iter()
        .zip(&req.validator_indices)
        .map(|(credential, index)| credential.bls_to_execution_change_entry(*index))
        .collect::<Result<_>>()?;

    ensure_directory(&req.folder)?;
    let path = req
        .folder
        .join(format!("bls_to_execution_change-{}.json", unix_timestamp()));
    write_sensitive_json(&path, &entries)?;

    if !verify_bls_to_execution_change_file(
        &path,
        &credentials,
        &req.validator_indices,
        withdrawal_address,
        chain,
    )? {
        return Err(Error::BtecVerification);
    }
    Ok(path)
}

/// Re-reads a `bls_to_execution_change-*.json` file and validates every entry.
pub fn verify_bls_to_execution_change_file(
    path: &Path,
    credentials: &[Credential],
    validator_indices: &[u64],
    withdrawal_address: Address,
    chain: &ChainSetting,
) -> Result<bool> {
    let file = std::fs::File::open(path).map_err(|e| Error::io("cannot open file", path, e))?;
    let entries: Vec<BtecEntry> = serde_json::from_reader(file)?;
    if entries.len() != credentials.len() || entries.len() != validator_indices.len() {
        return Ok(false);
    }
    Ok(entries.iter().zip(credentials).zip(validator_indices).all(
        |((entry, credential), index)| {
            validate_bls_to_execution_change(entry, credential, *index, withdrawal_address, chain)
        },
    ))
}

/// `validate_bls_to_execution_change` from the Python code.
pub fn validate_bls_to_execution_change(
    entry: &BtecEntry,
    credential: &Credential,
    input_validator_index: u64,
    input_withdrawal_address: Address,
    chain: &ChainSetting,
) -> bool {
    let Ok(validator_index) = entry.message.validator_index.parse::<u64>() else {
        return false;
    };
    let Some(from_bls_pubkey) = decode_hex(&entry.message.from_bls_pubkey)
        .and_then(|b| PublicKeyBytes::deserialize(&b).ok())
    else {
        return false;
    };
    let Some(to_execution_address) =
        decode_hex(&entry.message.to_execution_address).filter(|b| b.len() == 20)
    else {
        return false;
    };
    let Some(signature) =
        decode_hex(&entry.signature).and_then(|b| SignatureBytes::deserialize(&b).ok())
    else {
        return false;
    };
    let Some(genesis_validators_root) =
        decode_hex(&entry.metadata.genesis_validators_root).filter(|b| b.len() == 32)
    else {
        return false;
    };
    let to_execution_address = Address::from_slice(&to_execution_address);
    let genesis_validators_root = Hash256::from_slice(&genesis_validators_root);

    if validator_index != input_validator_index {
        return false;
    }
    if from_bls_pubkey != credential.withdrawal_pk() {
        return false;
    }
    if credential.withdrawal_address() != Some(to_execution_address)
        || to_execution_address != input_withdrawal_address
    {
        return false;
    }
    if chain.genesis_validators_root != Some(genesis_validators_root) {
        return false;
    }

    let message = BlsToExecutionChange {
        validator_index,
        from_bls_pubkey,
        to_execution_address,
    };
    let domain =
        compute_bls_to_execution_change_domain(chain.genesis_fork_version, genesis_validators_root);
    let signing_root = compute_signing_root(&message, domain);
    match (from_bls_pubkey.decompress(), signature.decompress()) {
        (Ok(pk), Ok(sig)) => sig.verify(&pk, signing_root),
        _ => false,
    }
}

fn derive_credentials(
    seed: &[u8],
    start_index: u32,
    count: usize,
    chain: &'static ChainSetting,
    withdrawal_address: Option<Address>,
) -> Result<Vec<Credential>> {
    let count = u32::try_from(count)
        .map_err(|_| Error::InvalidInput("Too many validators.".to_string()))?;
    let end = start_index
        .checked_add(count)
        .ok_or_else(|| Error::InvalidInput("Validator index out of range.".to_string()))?;
    let amount = chain.default_deposit_amount_gwei();
    (start_index..end)
        .into_par_iter()
        .map(|index| Credential::new(seed, index, amount, chain, withdrawal_address, false))
        .collect()
}

fn check_credentials_match(credentials: &[Credential], expected: &[Hash256]) -> Result<()> {
    for (credential, expected) in credentials.iter().zip(expected) {
        if credential.bls_withdrawal_credentials().as_slice()[1..] != expected.as_slice()[1..] {
            return Err(Error::BlsCredentialsMismatch);
        }
    }
    Ok(())
}

fn decode_hex(s: &str) -> Option<Vec<u8>> {
    hex::decode(s.strip_prefix("0x").unwrap_or(s)).ok()
}
