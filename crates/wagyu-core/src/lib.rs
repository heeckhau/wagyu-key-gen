//! Core library of Wagyu Key Gen.
//!
//! Produces the same files as `ethstaker-deposit-cli` 1.2.2 (EIP-2335 keystores, `deposit_data`
//! and `bls_to_execution_change` JSON) using Lighthouse's crypto crates for every cryptographic
//! operation. Nothing in this crate touches the UI, so it can be tested and reused on its own.

pub mod btec;
pub mod chain;
pub mod credential;
pub mod deposit;
pub mod error;
pub mod fs;
pub mod keystore;
pub mod mnemonic;
pub mod spec;
pub mod validation;

pub use btec::{
    generate_bls_to_execution_change, validate_bls_credentials, GenerateBtecRequest,
    ValidateBlsCredentialsRequest,
};
pub use chain::{ChainSetting, Network, DEPOSIT_CLI_VERSION};
pub use credential::{BtecEntry, Credential, DepositDatum};
pub use deposit::{generate_keys, GenerateKeysOutput, GenerateKeysRequest};
pub use error::{Error, Result};
pub use keystore::KdfChoice;
pub use mnemonic::{create_mnemonic, reconstruct_mnemonic, Language};
