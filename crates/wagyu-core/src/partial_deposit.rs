//! Top-up deposits for an existing validator, ported from the deposit-cli's `partial-deposit`
//! command. The deposit is signed with the key from a keystore file, so no mnemonic is needed.

use std::path::PathBuf;

use bls::{Hash256, SignatureBytes};
use zeroize::Zeroizing;

use crate::chain::{Network, MAX_DEPOSIT_AMOUNT_GWEI};
use crate::credential::{deposit_datum, DepositDatum};
use crate::deposit::validate_deposit;
use crate::error::{Error, Result};
use crate::fs::{ensure_directory, unix_timestamp, write_sensitive_json};
use crate::keystore::decrypt_keystore_file;
use crate::spec::{compute_deposit_domain, compute_signing_root, DepositData, DepositMessage};
use crate::validation::{
    parse_address, COMPOUNDING_WITHDRAWAL_PREFIX, EXECUTION_ADDRESS_WITHDRAWAL_PREFIX,
};

pub struct PartialDepositRequest {
    pub folder: PathBuf,
    pub network: Network,
    pub keystore_path: PathBuf,
    pub keystore_password: Zeroizing<String>,
    /// Deposit in gwei, already multiplied for Gnosis chains
    /// (see [`crate::validation::deposit_amount_to_gwei`]).
    pub amount_gwei: u64,
    /// Required: a top-up always carries `0x01` or `0x02` credentials.
    pub withdrawal_address: String,
    pub compounding: bool,
}

/// Writes a single-entry `deposit_data-<timestamp>.json` for the validator in the keystore and
/// re-reads it for verification. Returns the file path.
pub fn generate_partial_deposit(req: &PartialDepositRequest) -> Result<PathBuf> {
    let chain = req.network.setting();
    if req.amount_gwei < chain.min_deposit_message_amount_gwei()
        || req.amount_gwei > MAX_DEPOSIT_AMOUNT_GWEI
    {
        return Err(Error::InvalidAmount(
            "The deposit amount is not within the bounds of this cli.".to_string(),
        ));
    }
    let withdrawal_address = parse_address(&req.withdrawal_address)?;
    let keypair = decrypt_keystore_file(&req.keystore_path, req.keystore_password.as_bytes())?;

    let mut credentials = [0u8; 32];
    credentials[0] = if req.compounding {
        COMPOUNDING_WITHDRAWAL_PREFIX
    } else {
        EXECUTION_ADDRESS_WITHDRAWAL_PREFIX
    };
    credentials[12..].copy_from_slice(withdrawal_address.as_slice());

    let message = DepositMessage {
        pubkey: keypair.pk.compress(),
        withdrawal_credentials: Hash256::new(credentials),
        amount: req.amount_gwei,
    };
    let domain = compute_deposit_domain(chain.genesis_fork_version);
    let signing_root = compute_signing_root(&message, domain);
    let signed = DepositData {
        pubkey: message.pubkey,
        withdrawal_credentials: message.withdrawal_credentials,
        amount: message.amount,
        signature: SignatureBytes::from(keypair.sk.sign(signing_root)),
    };
    let datum = deposit_datum(&message, &signed, chain);

    ensure_directory(&req.folder)?;
    let path = req
        .folder
        .join(format!("deposit_data-{}.json", unix_timestamp()));
    write_sensitive_json(&path, &[datum])?;

    let file = std::fs::File::open(&path).map_err(|e| Error::io("cannot open file", &path, e))?;
    let written: Vec<DepositDatum> = serde_json::from_reader(file)?;
    if written.len() != 1 || !validate_deposit(&written[0], chain, None) {
        return Err(Error::DepositVerification);
    }
    Ok(path)
}
