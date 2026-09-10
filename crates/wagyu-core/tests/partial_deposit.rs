//! Port of `tests/test_cli/test_partial_deposit.py` plus golden parity with the Python output.

mod common;

use std::path::Path;

use common::{assert_mode_0400, golden_cases, read_json};
use serde_json::Value;
use wagyu_core::chain::Network;
use wagyu_core::credential::DepositDatum;
use wagyu_core::partial_deposit::{generate_partial_deposit, PartialDepositRequest};
use wagyu_core::Error;

const MAINNET_KEYSTORE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/vectors/golden/keys_mainnet_bls_3/keystore-0.json"
);
const PUBKEY: &str = "b3e445d43871965d890a398f719348a1405ac72e35b92727cc570026f54471af7ea7b2040622a8fd0b5bfb2a209b5911";
const ADDRESS: &str = "0x3434343434343434343434343434343434343434";

fn request(folder: &Path) -> PartialDepositRequest {
    PartialDepositRequest {
        folder: folder.to_path_buf(),
        network: Network::Mainnet,
        keystore_path: MAINNET_KEYSTORE.into(),
        keystore_password: "MyPasswordIs".to_string().into(),
        amount_gwei: 1_000_000_000,
        withdrawal_address: ADDRESS.to_string(),
        compounding: false,
    }
}

fn read_deposits(path: &Path) -> Vec<DepositDatum> {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn test_partial_deposit() {
    let dir = tempfile::tempdir().unwrap();
    let path = generate_partial_deposit(&request(dir.path())).unwrap();
    assert!(path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("deposit_data-"));
    assert_mode_0400(&path);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);

    let deposits = read_deposits(&path);
    assert_eq!(deposits.len(), 1);
    let deposit = &deposits[0];
    assert_eq!(deposit.pubkey, PUBKEY);
    assert_eq!(deposit.amount, 1_000_000_000);
    assert_eq!(
        deposit.withdrawal_credentials,
        format!("01{}{}", "00".repeat(11), &ADDRESS[2..])
    );
    assert_eq!(deposit.network_name, "mainnet");
    assert_eq!(deposit.fork_version, "00000000");
    assert_eq!(deposit.deposit_cli_version, "1.2.2");
}

#[test]
fn compounding_uses_the_0x02_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let mut req = request(dir.path());
    req.compounding = true;
    req.amount_gwei = 2_048_000_000_000;
    let deposit = read_deposits(&generate_partial_deposit(&req).unwrap()).remove(0);
    assert!(deposit.withdrawal_credentials.starts_with("02"));
    assert_eq!(deposit.amount, 2_048_000_000_000);
}

#[test]
fn test_errors() {
    let dir = tempfile::tempdir().unwrap();

    let mut req = request(dir.path());
    req.amount_gwei = 999_999_999;
    assert!(matches!(
        generate_partial_deposit(&req),
        Err(Error::InvalidAmount(_))
    ));

    let mut req = request(dir.path());
    req.amount_gwei = 2_048_000_000_001;
    assert!(matches!(
        generate_partial_deposit(&req),
        Err(Error::InvalidAmount(_))
    ));

    let mut req = request(dir.path());
    req.withdrawal_address = String::new();
    assert!(matches!(
        generate_partial_deposit(&req),
        Err(Error::InvalidAddress)
    ));

    let mut req = request(dir.path());
    req.keystore_password = "wrong password".to_string().into();
    assert!(matches!(
        generate_partial_deposit(&req),
        Err(Error::WrongKeystorePassword)
    ));

    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        0,
        "no files written on error"
    );
}

/// The BLS signature is deterministic, so the file must be identical to the Python output.
#[test]
fn golden_parity() {
    for case in golden_cases("partial_deposit") {
        let name = case.file_name().unwrap().to_str().unwrap().to_string();
        let params = read_json(&case.join("params.json"));
        let dir = tempfile::tempdir().unwrap();
        let req = PartialDepositRequest {
            folder: dir.path().to_path_buf(),
            network: Network::from_name(params["network"].as_str().unwrap()).unwrap(),
            keystore_path: case.join(params["keystore"].as_str().unwrap()),
            keystore_password: params["password"].as_str().unwrap().to_string().into(),
            amount_gwei: params["amount_gwei"].as_u64().unwrap(),
            withdrawal_address: params["withdrawal_address"].as_str().unwrap().to_string(),
            compounding: params["compounding"].as_bool().unwrap(),
        };
        let path = generate_partial_deposit(&req).unwrap_or_else(|e| panic!("{name}: {e}"));
        let expected: Value = read_json(&case.join("deposit_data.json"));
        let actual: Value = read_json(&path);
        assert_eq!(actual, expected, "{name}: deposit_data.json differs");
    }
}
