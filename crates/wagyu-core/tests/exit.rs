//! Port of `tests/test_cli/test_exit_transaction_mnemonic.py` and
//! `test_exit_transaction_keystore.py`, plus golden parity with the Python output.

mod common;

use std::path::Path;

use common::{assert_mode_0400, golden_cases, read_json, SISTER};
use serde_json::Value;
use wagyu_core::chain::Network;
use wagyu_core::exit::{
    generate_exit_transaction_from_keystore, generate_exit_transactions, ExitTransaction,
    GenerateExitFromKeystoreRequest, GenerateExitFromMnemonicRequest,
};
use wagyu_core::validation::parse_validator_indices;
use wagyu_core::Error;

const MAINNET_KEYSTORE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/vectors/golden/keys_mainnet_bls_3/keystore-0.json"
);
const PASSWORD: &str = "MyPasswordIs";

fn mnemonic_request(folder: &Path) -> GenerateExitFromMnemonicRequest {
    GenerateExitFromMnemonicRequest {
        folder: folder.to_path_buf(),
        network: Network::Mainnet,
        mnemonic: SISTER.to_string().into(),
        mnemonic_password: String::new().into(),
        mnemonic_language: None,
        start_index: 0,
        validator_indices: vec![1, 2],
        epoch: 1234,
    }
}

fn keystore_request(folder: &Path) -> GenerateExitFromKeystoreRequest {
    GenerateExitFromKeystoreRequest {
        folder: folder.to_path_buf(),
        network: Network::Mainnet,
        keystore_path: MAINNET_KEYSTORE.into(),
        keystore_password: PASSWORD.to_string().into(),
        validator_index: 1,
        epoch: 1234,
    }
}

fn read_transaction(path: &Path) -> ExitTransaction {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn test_exit_transaction_mnemonic() {
    let dir = tempfile::tempdir().unwrap();
    let paths = generate_exit_transactions(&mnemonic_request(dir.path())).unwrap();
    assert_eq!(paths.len(), 2);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    for (path, index) in paths.iter().zip(["1", "2"]) {
        let name = path.file_name().unwrap().to_str().unwrap();
        assert!(
            name.starts_with(&format!("signed_exit_transaction-{index}-")),
            "{name}"
        );
        assert_mode_0400(path);
        let transaction = read_transaction(path);
        assert_eq!(transaction.message.validator_index, index);
        assert_eq!(transaction.message.epoch, "1234");
        assert!(transaction.signature.starts_with("0x") && transaction.signature.len() == 2 + 192);
    }
}

#[test]
fn test_exit_transaction_keystore() {
    let dir = tempfile::tempdir().unwrap();
    let path = generate_exit_transaction_from_keystore(&keystore_request(dir.path())).unwrap();
    assert!(path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("signed_exit_transaction-1-"));
    assert_mode_0400(&path);
    let transaction = read_transaction(&path);
    assert_eq!(transaction.message.validator_index, "1");
    assert_eq!(transaction.message.epoch, "1234");
}

#[test]
fn test_errors() {
    let dir = tempfile::tempdir().unwrap();

    let mut req = mnemonic_request(dir.path());
    req.validator_indices = vec![];
    assert!(matches!(
        generate_exit_transactions(&req),
        Err(Error::InvalidInput(_))
    ));

    let mut req = mnemonic_request(dir.path());
    req.network = Network::Ephemery;
    assert!(matches!(
        generate_exit_transactions(&req),
        Err(Error::MissingGenesisValidatorsRoot)
    ));

    let mut req = mnemonic_request(dir.path());
    req.mnemonic = "abandon abandon".to_string().into();
    assert!(matches!(
        generate_exit_transactions(&req),
        Err(Error::InvalidMnemonic)
    ));

    let mut req = keystore_request(dir.path());
    req.keystore_password = "wrong password".to_string().into();
    assert!(matches!(
        generate_exit_transaction_from_keystore(&req),
        Err(Error::WrongKeystorePassword)
    ));

    let not_a_keystore = dir.path().join("deposit_data-1.json");
    std::fs::write(&not_a_keystore, "[]").unwrap();
    let mut req = keystore_request(dir.path());
    req.keystore_path = not_a_keystore.clone();
    assert!(matches!(
        generate_exit_transaction_from_keystore(&req),
        Err(Error::InvalidKeystoreFile(p)) if p == not_a_keystore
    ));

    let mut req = keystore_request(dir.path());
    req.keystore_path = dir.path().join("missing.json");
    assert!(matches!(
        generate_exit_transaction_from_keystore(&req),
        Err(Error::InvalidKeystoreFile(_))
    ));

    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        1,
        "no exit files written on error"
    );
}

/// The BLS signature is deterministic, so the files must be identical to the Python output.
#[test]
fn golden_parity_mnemonic() {
    for case in golden_cases("exit_transaction_mnemonic") {
        let name = case.file_name().unwrap().to_str().unwrap().to_string();
        let params = read_json(&case.join("params.json"));
        let dir = tempfile::tempdir().unwrap();
        let validator_indices =
            parse_validator_indices(params["indices"].as_str().unwrap()).unwrap();
        let req = GenerateExitFromMnemonicRequest {
            folder: dir.path().to_path_buf(),
            network: Network::from_name(params["network"].as_str().unwrap()).unwrap(),
            mnemonic: params["mnemonic"].as_str().unwrap().to_string().into(),
            mnemonic_password: String::new().into(),
            mnemonic_language: None,
            start_index: params["index"].as_u64().unwrap() as u32,
            validator_indices: validator_indices.clone(),
            epoch: params["epoch"].as_u64().unwrap(),
        };
        let paths = generate_exit_transactions(&req).unwrap_or_else(|e| panic!("{name}: {e}"));
        for (path, index) in paths.iter().zip(validator_indices) {
            let expected: Value =
                read_json(&case.join(format!("signed_exit_transaction-{index}.json")));
            let actual: Value = read_json(path);
            assert_eq!(actual, expected, "{name}: validator {index} differs");
        }
    }
}

#[test]
fn golden_parity_keystore() {
    for case in golden_cases("exit_transaction_keystore") {
        let name = case.file_name().unwrap().to_str().unwrap().to_string();
        let params = read_json(&case.join("params.json"));
        let dir = tempfile::tempdir().unwrap();
        let req = GenerateExitFromKeystoreRequest {
            folder: dir.path().to_path_buf(),
            network: Network::from_name(params["network"].as_str().unwrap()).unwrap(),
            keystore_path: case.join(params["keystore"].as_str().unwrap()),
            keystore_password: params["password"].as_str().unwrap().to_string().into(),
            validator_index: params["validator_index"].as_u64().unwrap(),
            epoch: params["epoch"].as_u64().unwrap(),
        };
        let path =
            generate_exit_transaction_from_keystore(&req).unwrap_or_else(|e| panic!("{name}: {e}"));
        let expected: Value = read_json(&case.join("signed_exit_transaction.json"));
        let actual: Value = read_json(&path);
        assert_eq!(
            actual, expected,
            "{name}: signed_exit_transaction.json differs"
        );
    }
}
