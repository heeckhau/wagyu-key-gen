//! Port of `tests/test_cli/test_generate_bls_to_execution_change.py` plus parity with the Python
//! proxy's `bls_change` and `validate_bls_credentials` subcommands.

mod common;

use std::path::Path;

use common::{assert_mode_0400, golden_cases, read_json, SISTER};
use serde_json::Value;
use wagyu_core::btec::{
    generate_bls_to_execution_change, generate_bls_to_execution_change_keystore,
    validate_bls_credentials, BtecKeystoreSignature, GenerateBtecKeystoreRequest,
    GenerateBtecRequest, ValidateBlsCredentialsRequest,
};
use wagyu_core::chain::Network;
use wagyu_core::credential::BtecEntry;
use wagyu_core::validation::{parse_bls_withdrawal_credentials_list, parse_validator_indices};
use wagyu_core::Error;

const CREDS: &str = "0x00bd0b5a34de5fb17df08410b5e615dda87caf4fb72d0aac91ce5e52fc6aa8de,0x00a75d83f169fa6923f3dd78386d9608fab710d8f7fcf71ba9985893675d5382";
const ADDRESS: &str = "0x3434343434343434343434343434343434343434";

fn request(folder: &Path) -> GenerateBtecRequest {
    GenerateBtecRequest {
        folder: folder.to_path_buf(),
        network: Network::Mainnet,
        mnemonic: SISTER.to_string().into(),
        mnemonic_password: String::new().into(),
        mnemonic_language: None,
        start_index: 0,
        validator_indices: vec![1, 2],
        bls_withdrawal_credentials: parse_bls_withdrawal_credentials_list(CREDS).unwrap(),
        withdrawal_address: ADDRESS.to_string(),
    }
}

fn read_entries(path: &Path) -> Vec<BtecEntry> {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn test_existing_mnemonic_bls_withdrawal_multiple() {
    let dir = tempfile::tempdir().unwrap();
    let req = request(dir.path());
    let path = generate_bls_to_execution_change(&req).unwrap();
    assert!(path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("bls_to_execution_change-"));
    assert_mode_0400(&path);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    let entries = read_entries(&path);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].message.validator_index, "1");
    assert_eq!(entries[1].message.validator_index, "2");
    for entry in &entries {
        assert_eq!(entry.message.to_execution_address, ADDRESS);
        assert_eq!(entry.metadata.network_name, "mainnet");
        assert_eq!(entry.metadata.deposit_cli_version, "1.2.2");
        assert!(entry.signature.starts_with("0x") && entry.signature.len() == 2 + 192);
    }
}

#[test]
fn test_single_validator_with_start_index() {
    let dir = tempfile::tempdir().unwrap();
    let mut req = request(dir.path());
    req.start_index = 1;
    req.validator_indices = vec![7];
    req.bls_withdrawal_credentials =
        parse_bls_withdrawal_credentials_list(CREDS.split(',').nth(1).unwrap()).unwrap();
    let path = generate_bls_to_execution_change(&req).unwrap();
    assert_eq!(read_entries(&path)[0].message.validator_index, "7");
}

#[test]
fn test_validate_bls_credentials() {
    let ok = ValidateBlsCredentialsRequest {
        network: Network::Mainnet,
        mnemonic: SISTER.to_string().into(),
        mnemonic_password: String::new().into(),
        mnemonic_language: None,
        start_index: 0,
        bls_withdrawal_credentials: parse_bls_withdrawal_credentials_list(CREDS).unwrap(),
    };
    validate_bls_credentials(&ok).unwrap();

    let mut wrong_index = ok;
    wrong_index.start_index = 1;
    assert!(matches!(
        validate_bls_credentials(&wrong_index),
        Err(Error::BlsCredentialsMismatch)
    ));
}

#[test]
fn test_errors() {
    let dir = tempfile::tempdir().unwrap();

    let mut req = request(dir.path());
    req.validator_indices = vec![1];
    assert!(matches!(
        generate_bls_to_execution_change(&req),
        Err(Error::IndicesCredentialsMismatch)
    ));

    let mut req = request(dir.path());
    req.start_index = 3;
    assert!(matches!(
        generate_bls_to_execution_change(&req),
        Err(Error::BlsCredentialsMismatch)
    ));

    let mut req = request(dir.path());
    req.withdrawal_address = "0x3434".to_string();
    assert!(matches!(
        generate_bls_to_execution_change(&req),
        Err(Error::InvalidAddress)
    ));

    let mut req = request(dir.path());
    req.network = Network::Ephemery;
    assert!(matches!(
        generate_bls_to_execution_change(&req),
        Err(Error::MissingGenesisValidatorsRoot)
    ));

    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        0,
        "no files written on error"
    );
}

const MAINNET_KEYSTORE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/vectors/golden/keys_mainnet_bls_3/keystore-0.json"
);

fn keystore_request(folder: &Path) -> GenerateBtecKeystoreRequest {
    GenerateBtecKeystoreRequest {
        folder: folder.to_path_buf(),
        network: Network::Mainnet,
        keystore_path: MAINNET_KEYSTORE.into(),
        keystore_password: "MyPasswordIs".to_string().into(),
        validator_index: 1,
        withdrawal_address: ADDRESS.to_string(),
    }
}

#[test]
fn test_bls_to_execution_change_keystore() {
    let dir = tempfile::tempdir().unwrap();
    let path = generate_bls_to_execution_change_keystore(&keystore_request(dir.path())).unwrap();
    assert!(path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("bls_to_execution_change_keystore_signature-1-"));
    assert_mode_0400(&path);
    let entry: BtecKeystoreSignature =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(entry.message.validator_index, 1);
    assert_eq!(entry.message.to_execution_address, ADDRESS);
    assert!(entry.signature.starts_with("0x") && entry.signature.len() == 2 + 192);

    let mut wrong_password = keystore_request(dir.path());
    wrong_password.keystore_password = "nope nope nope".to_string().into();
    assert!(matches!(
        generate_bls_to_execution_change_keystore(&wrong_password),
        Err(Error::WrongKeystorePassword)
    ));
    let mut bad_address = keystore_request(dir.path());
    bad_address.withdrawal_address = "0x3434".to_string();
    assert!(matches!(
        generate_bls_to_execution_change_keystore(&bad_address),
        Err(Error::InvalidAddress)
    ));
    let mut ephemery = keystore_request(dir.path());
    ephemery.network = Network::Ephemery;
    assert!(matches!(
        generate_bls_to_execution_change_keystore(&ephemery),
        Err(Error::MissingGenesisValidatorsRoot)
    ));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn golden_parity_keystore() {
    for case in golden_cases("bls_change_keystore") {
        let name = case.file_name().unwrap().to_str().unwrap().to_string();
        let params = read_json(&case.join("params.json"));
        let dir = tempfile::tempdir().unwrap();
        let req = GenerateBtecKeystoreRequest {
            folder: dir.path().to_path_buf(),
            network: Network::from_name(params["network"].as_str().unwrap()).unwrap(),
            keystore_path: case.join(params["keystore"].as_str().unwrap()),
            keystore_password: params["password"].as_str().unwrap().to_string().into(),
            validator_index: params["validator_index"].as_u64().unwrap(),
            withdrawal_address: params["withdrawal_address"].as_str().unwrap().to_string(),
        };
        let path = generate_bls_to_execution_change_keystore(&req)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let expected: Value =
            read_json(&case.join("bls_to_execution_change_keystore_signature.json"));
        let actual: Value = read_json(&path);
        assert_eq!(actual, expected, "{name}: signature file differs");
    }
}

/// The BLS signature is deterministic, so the file must be identical to the Python output.
#[test]
fn golden_parity() {
    for case in golden_cases("bls_change") {
        let name = case.file_name().unwrap().to_str().unwrap().to_string();
        let params = read_json(&case.join("params.json"));
        let dir = tempfile::tempdir().unwrap();
        let req = GenerateBtecRequest {
            folder: dir.path().to_path_buf(),
            network: Network::from_name(params["network"].as_str().unwrap()).unwrap(),
            mnemonic: params["mnemonic"].as_str().unwrap().to_string().into(),
            mnemonic_password: String::new().into(),
            mnemonic_language: None,
            start_index: params["index"].as_u64().unwrap() as u32,
            validator_indices: parse_validator_indices(params["indices"].as_str().unwrap())
                .unwrap(),
            bls_withdrawal_credentials: parse_bls_withdrawal_credentials_list(
                params["withdrawal_credentials"].as_str().unwrap(),
            )
            .unwrap(),
            withdrawal_address: params["execution_address"].as_str().unwrap().to_string(),
        };
        let path = generate_bls_to_execution_change(&req).unwrap_or_else(|e| panic!("{name}: {e}"));
        let expected: Value = read_json(&case.join("bls_to_execution_change.json"));
        let actual: Value = read_json(&path);
        assert_eq!(
            actual, expected,
            "{name}: bls_to_execution_change.json differs"
        );
    }
}
