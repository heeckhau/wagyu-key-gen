//! One validator's keys and everything derived from them, ported from
//! `ethstaker_deposit/credentials.py`.

use alloy_primitives::Address;
use bls::{get_withdrawal_credentials, Hash256, Keypair, PublicKeyBytes, SignatureBytes};
use eth2_keystore::{keypair_from_secret, Keystore};
use eth2_wallet::{recover_validator_secret_from_mnemonic, KeyType, ValidatorPath};
use tree_hash::TreeHash;

use crate::chain::{ChainSetting, DEPOSIT_CLI_VERSION, GWEI_PER_ETH, MAX_DEPOSIT_AMOUNT_GWEI};
use crate::error::{Error, Result};
use crate::keystore::{decrypt_keystore, encrypt_keystore, KdfChoice};
use crate::spec::{
    compute_bls_to_execution_change_domain, compute_deposit_domain, compute_signing_root,
    BlsToExecutionChange, DepositData, DepositMessage, SignedBlsToExecutionChange,
    SignedVoluntaryExit,
};
use crate::validation::{
    BLS_WITHDRAWAL_PREFIX, COMPOUNDING_WITHDRAWAL_PREFIX, EXECUTION_ADDRESS_WITHDRAWAL_PREFIX,
};

/// Keys and settings for the validator at one EIP-2334 index.
pub struct Credential {
    index: u32,
    signing: Keypair,
    withdrawal: Keypair,
    chain: &'static ChainSetting,
    withdrawal_address: Option<Address>,
    compounding: bool,
    amount_gwei: u64,
}

impl Credential {
    /// Derives the withdrawal key (`m/12381/3600/<index>/0`) and signing key
    /// (`m/12381/3600/<index>/0/0`) from a BIP-39 `seed`.
    pub fn new(
        seed: &[u8],
        index: u32,
        amount_gwei: u64,
        chain: &'static ChainSetting,
        withdrawal_address: Option<Address>,
        compounding: bool,
    ) -> Result<Self> {
        if compounding && withdrawal_address.is_none() {
            return Err(Error::MissingAddress);
        }
        let (withdrawal_secret, _) =
            recover_validator_secret_from_mnemonic(seed, index, KeyType::Withdrawal)?;
        let (signing_secret, _) =
            recover_validator_secret_from_mnemonic(seed, index, KeyType::Voting)?;
        Ok(Credential {
            index,
            signing: keypair_from_secret(signing_secret.as_bytes())?,
            withdrawal: keypair_from_secret(withdrawal_secret.as_bytes())?,
            chain,
            withdrawal_address,
            compounding,
            amount_gwei,
        })
    }

    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn chain(&self) -> &'static ChainSetting {
        self.chain
    }

    pub fn amount_gwei(&self) -> u64 {
        self.amount_gwei
    }

    pub fn withdrawal_address(&self) -> Option<Address> {
        self.withdrawal_address
    }

    /// `m/12381/3600/<index>/0/0`
    pub fn signing_key_path(&self) -> String {
        ValidatorPath::new(self.index, KeyType::Voting).to_string()
    }

    pub fn signing_pk(&self) -> PublicKeyBytes {
        self.signing.pk.compress()
    }

    pub fn withdrawal_pk(&self) -> PublicKeyBytes {
        self.withdrawal.pk.compress()
    }

    /// `0x00` (BLS), `0x01` (execution address) or `0x02` (compounding).
    pub fn withdrawal_prefix(&self) -> u8 {
        match (self.withdrawal_address, self.compounding) {
            (None, _) => BLS_WITHDRAWAL_PREFIX,
            (Some(_), false) => EXECUTION_ADDRESS_WITHDRAWAL_PREFIX,
            (Some(_), true) => COMPOUNDING_WITHDRAWAL_PREFIX,
        }
    }

    /// The `0x00 ‖ sha256(withdrawal_pk)[1..]` credentials, regardless of the configured type.
    pub fn bls_withdrawal_credentials(&self) -> Hash256 {
        Hash256::from_slice(&get_withdrawal_credentials(
            &self.withdrawal.pk,
            BLS_WITHDRAWAL_PREFIX,
        ))
    }

    pub fn withdrawal_credentials(&self) -> Hash256 {
        match self.withdrawal_address {
            None => self.bls_withdrawal_credentials(),
            Some(address) => {
                let mut bytes = [0u8; 32];
                bytes[0] = self.withdrawal_prefix();
                bytes[12..].copy_from_slice(address.as_slice());
                Hash256::new(bytes)
            }
        }
    }

    pub fn deposit_message(&self) -> Result<DepositMessage> {
        let min = self.chain.min_deposit_message_amount_gwei();
        if self.amount_gwei < min || self.amount_gwei > MAX_DEPOSIT_AMOUNT_GWEI {
            return Err(Error::InvalidAmount(format!(
                "{} ETH deposits are not within the bounds of this cli.",
                self.amount_gwei as f64 / GWEI_PER_ETH as f64
            )));
        }
        Ok(DepositMessage {
            pubkey: self.signing_pk(),
            withdrawal_credentials: self.withdrawal_credentials(),
            amount: self.amount_gwei,
        })
    }

    pub fn signed_deposit(&self) -> Result<DepositData> {
        let message = self.deposit_message()?;
        let domain = compute_deposit_domain(self.chain.genesis_fork_version);
        let signing_root = compute_signing_root(&message, domain);
        let signature = self.signing.sk.sign(signing_root);
        Ok(DepositData {
            pubkey: message.pubkey,
            withdrawal_credentials: message.withdrawal_credentials,
            amount: message.amount,
            signature: SignatureBytes::from(signature),
        })
    }

    /// One entry of the `deposit_data-*.json` file.
    pub fn deposit_datum(&self) -> Result<DepositDatum> {
        let message = self.deposit_message()?;
        let signed = self.signed_deposit()?;
        Ok(deposit_datum(&message, &signed, self.chain))
    }

    pub fn signed_voluntary_exit(
        &self,
        validator_index: u64,
        epoch: u64,
    ) -> Result<SignedVoluntaryExit> {
        crate::exit::sign_voluntary_exit(&self.signing.sk, self.chain, validator_index, epoch)
    }

    pub fn signing_keystore(&self, password: &[u8], kdf: KdfChoice) -> Result<Keystore> {
        encrypt_keystore(&self.signing, password, self.signing_key_path(), kdf)
    }

    /// True if `keystore` decrypts with `password` to this credential's signing key.
    pub fn verify_keystore(&self, keystore: &Keystore, password: &[u8]) -> Result<bool> {
        let keypair = decrypt_keystore(keystore, password)?;
        Ok(keypair.sk.serialize().as_bytes() == self.signing.sk.serialize().as_bytes())
    }

    pub fn bls_to_execution_change(
        &self,
        validator_index: u64,
    ) -> Result<SignedBlsToExecutionChange> {
        let to_execution_address = self.withdrawal_address.ok_or(Error::MissingAddress)?;
        let genesis_validators_root = self
            .chain
            .genesis_validators_root
            .ok_or(Error::MissingGenesisValidatorsRoot)?;
        let message = BlsToExecutionChange {
            validator_index,
            from_bls_pubkey: self.withdrawal_pk(),
            to_execution_address,
        };
        let domain = compute_bls_to_execution_change_domain(
            self.chain.genesis_fork_version,
            genesis_validators_root,
        );
        let signing_root = compute_signing_root(&message, domain);
        let signature = self.withdrawal.sk.sign(signing_root);
        Ok(SignedBlsToExecutionChange {
            message,
            signature: SignatureBytes::from(signature),
        })
    }

    /// One entry of the `bls_to_execution_change-*.json` file.
    pub fn bls_to_execution_change_entry(&self, validator_index: u64) -> Result<BtecEntry> {
        let signed = self.bls_to_execution_change(validator_index)?;
        let genesis_validators_root = self
            .chain
            .genesis_validators_root
            .ok_or(Error::MissingGenesisValidatorsRoot)?;
        Ok(BtecEntry {
            message: BtecMessage {
                validator_index: signed.message.validator_index.to_string(),
                from_bls_pubkey: format!(
                    "0x{}",
                    hex::encode(signed.message.from_bls_pubkey.serialize())
                ),
                to_execution_address: format!(
                    "0x{}",
                    hex::encode(signed.message.to_execution_address)
                ),
            },
            signature: format!("0x{}", hex::encode(signed.signature.serialize())),
            metadata: BtecMetadata {
                network_name: self.chain.network_name.to_string(),
                genesis_validators_root: format!("0x{}", hex::encode(genesis_validators_root)),
                deposit_cli_version: DEPOSIT_CLI_VERSION.to_string(),
            },
        })
    }
}

/// Lays out a signed deposit as one `deposit_data-*.json` entry.
pub fn deposit_datum(
    message: &DepositMessage,
    signed: &DepositData,
    chain: &ChainSetting,
) -> DepositDatum {
    DepositDatum {
        pubkey: hex::encode(signed.pubkey.serialize()),
        withdrawal_credentials: hex::encode(signed.withdrawal_credentials),
        amount: signed.amount,
        signature: hex::encode(signed.signature.serialize()),
        deposit_message_root: hex::encode(message.tree_hash_root()),
        deposit_data_root: hex::encode(signed.tree_hash_root()),
        fork_version: hex::encode(chain.genesis_fork_version),
        network_name: chain.network_name.to_string(),
        deposit_cli_version: DEPOSIT_CLI_VERSION.to_string(),
    }
}

/// JSON layout of one deposit in `deposit_data-*.json` (hex fields have no `0x` prefix).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DepositDatum {
    pub pubkey: String,
    pub withdrawal_credentials: String,
    pub amount: u64,
    pub signature: String,
    pub deposit_message_root: String,
    pub deposit_data_root: String,
    pub fork_version: String,
    pub network_name: String,
    pub deposit_cli_version: String,
}

/// JSON layout of one entry in `bls_to_execution_change-*.json` (hex fields carry `0x`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BtecEntry {
    pub message: BtecMessage,
    pub signature: String,
    pub metadata: BtecMetadata,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BtecMessage {
    pub validator_index: String,
    pub from_bls_pubkey: String,
    pub to_execution_address: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BtecMetadata {
    pub network_name: String,
    pub genesis_validators_root: String,
    pub deposit_cli_version: String,
}
