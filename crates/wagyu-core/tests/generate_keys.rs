//! Port of `tests/test_cli/test_new_mnemonic.py`, `test_existing_mnemonic.py` and
//! `test_regeneration.py`, plus byte-level parity with files produced by the Python proxy.

mod common;

use std::path::Path;

use common::{assert_mode_0400, golden_cases, golden_dir, read_json, ABANDON};
use eth2_keystore::Keystore;
use serde_json::Value;
use wagyu_core::chain::{Network, GWEI_PER_ETH};
use wagyu_core::credential::DepositDatum;
use wagyu_core::deposit::{
    generate_keys, validate_deposit, GenerateKeysOutput, GenerateKeysRequest,
};
use wagyu_core::keystore::KdfChoice;
use wagyu_core::mnemonic::Language;
use wagyu_core::Error;

const PASSWORD: &str = "MyPasswordIs";
const ADDRESS: &str = "0x00000000219ab540356cBB839Cbe05303d7705Fa";

fn request(folder: &Path) -> GenerateKeysRequest {
    GenerateKeysRequest {
        mnemonic: ABANDON.to_string().into(),
        mnemonic_password: String::new().into(),
        mnemonic_language: None,
        start_index: 0,
        count: 1,
        amount_gwei: 32 * GWEI_PER_ETH,
        network: Network::Mainnet,
        keystore_password: PASSWORD.to_string().into(),
        withdrawal_address: None,
        compounding: false,
        folder: folder.to_path_buf(),
        kdf: KdfChoice::Scrypt,
    }
}

fn request_from_params(params: &Value, folder: &Path) -> GenerateKeysRequest {
    GenerateKeysRequest {
        mnemonic: params["mnemonic"].as_str().unwrap().to_string().into(),
        mnemonic_password: String::new().into(),
        mnemonic_language: None,
        start_index: params["index"].as_u64().unwrap() as u32,
        count: params["count"].as_u64().unwrap() as u32,
        amount_gwei: params["amount_gwei"].as_u64().unwrap(),
        network: Network::from_name(params["network"].as_str().unwrap()).unwrap(),
        keystore_password: params["password"].as_str().unwrap().to_string().into(),
        withdrawal_address: params["eth1_withdrawal_address"]
            .as_str()
            .map(str::to_string),
        compounding: params["compounding"].as_bool().unwrap(),
        folder: folder.to_path_buf(),
        kdf: KdfChoice::Scrypt,
    }
}

fn read_deposits(path: &Path) -> Vec<DepositDatum> {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn keystore_files(folder: &Path) -> Vec<std::path::PathBuf> {
    let mut files: Vec<_> = std::fs::read_dir(folder)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("keystore-")
        })
        .collect();
    files.sort();
    files
}

fn check_output(req: &GenerateKeysRequest, out: &GenerateKeysOutput) {
    assert_eq!(out.keystore_files.len(), req.count as usize);
    assert_eq!(out.pubkeys.len(), req.count as usize);
    let files = keystore_files(&req.folder);
    assert_eq!(files.len(), req.count as usize);
    let mut uuids = std::collections::HashSet::new();
    for (i, file) in out.keystore_files.iter().enumerate() {
        assert_mode_0400(file);
        let name = file.file_name().unwrap().to_str().unwrap();
        assert!(
            name.starts_with(&format!(
                "keystore-m_12381_3600_{}_0_0-",
                req.start_index + i as u32
            )),
            "{name}"
        );
        let keystore = Keystore::from_json_file(file).unwrap();
        assert!(uuids.insert(*keystore.uuid()));
        assert_eq!(keystore.pubkey(), out.pubkeys[i]);
        assert_eq!(
            keystore.path().unwrap(),
            format!("m/12381/3600/{}/0/0", req.start_index + i as u32)
        );
        assert_eq!(keystore.description(), Some(""));
    }
    assert_mode_0400(&out.deposit_data_file);
    let deposits = read_deposits(&out.deposit_data_file);
    assert_eq!(deposits.len(), req.count as usize);
    let chain = req.network.setting();
    for (i, deposit) in deposits.iter().enumerate() {
        assert_eq!(deposit.pubkey, out.pubkeys[i]);
        assert_eq!(deposit.amount, req.amount_gwei);
        assert_eq!(deposit.network_name, chain.network_name);
        assert_eq!(
            deposit.fork_version,
            hex::encode(chain.genesis_fork_version)
        );
        assert_eq!(deposit.deposit_cli_version, "1.2.2");
        assert!(validate_deposit(deposit, chain, None));
    }
}

fn expected_credentials(prefix: u8, address: Option<&str>) -> String {
    let mut bytes = [0u8; 32];
    bytes[0] = prefix;
    if let Some(address) = address {
        bytes[12..].copy_from_slice(&hex::decode(&address[2..]).unwrap());
    }
    hex::encode(bytes)
}

#[test]
fn test_new_mnemonic_bls_withdrawal() {
    let dir = tempfile::tempdir().unwrap();
    let req = request(dir.path());
    let out = generate_keys(&req).unwrap();
    check_output(&req, &out);
    for d in read_deposits(&out.deposit_data_file) {
        assert!(d.withdrawal_credentials.starts_with("00"));
        assert_eq!(d.amount, 32 * GWEI_PER_ETH);
    }
}

#[test]
fn test_new_mnemonic_withdrawal_address() {
    let dir = tempfile::tempdir().unwrap();
    let mut req = request(dir.path());
    req.withdrawal_address = Some(ADDRESS.to_string());
    let out = generate_keys(&req).unwrap();
    check_output(&req, &out);
    for d in read_deposits(&out.deposit_data_file) {
        assert_eq!(
            d.withdrawal_credentials,
            expected_credentials(0x01, Some(ADDRESS))
        );
    }
}

#[test]
fn test_new_mnemonic_compounding_custom_amount() {
    let dir = tempfile::tempdir().unwrap();
    let mut req = request(dir.path());
    req.withdrawal_address = Some(ADDRESS.to_string());
    req.compounding = true;
    req.amount_gwei = 1050 * GWEI_PER_ETH;
    let out = generate_keys(&req).unwrap();
    check_output(&req, &out);
    for d in read_deposits(&out.deposit_data_file) {
        assert_eq!(
            d.withdrawal_credentials,
            expected_credentials(0x02, Some(ADDRESS))
        );
        assert_eq!(d.amount, 1050 * GWEI_PER_ETH);
    }
}

#[test]
fn test_gnosis_and_chiado_compounding() {
    for network in [Network::Gnosis, Network::Chiado] {
        let dir = tempfile::tempdir().unwrap();
        let mut req = request(dir.path());
        req.network = network;
        req.withdrawal_address = Some(ADDRESS.to_string());
        req.compounding = true;
        req.amount_gwei = network.setting().default_deposit_amount_gwei();
        let out = generate_keys(&req).unwrap();
        check_output(&req, &out);
        for d in read_deposits(&out.deposit_data_file) {
            assert_eq!(d.amount, 32 * GWEI_PER_ETH);
            assert_eq!(
                d.withdrawal_credentials,
                expected_credentials(0x02, Some(ADDRESS))
            );
        }
    }
}

#[test]
fn test_withdrawal_address_bad_checksum() {
    let dir = tempfile::tempdir().unwrap();
    let mut req = request(dir.path());
    req.withdrawal_address = Some("0x00000000219ab540356cBB839Cbe05303d7705FA".to_string());
    assert!(matches!(generate_keys(&req), Err(Error::InvalidAddress)));
    assert!(
        keystore_files(dir.path()).is_empty(),
        "no files written on error"
    );
}

#[test]
fn test_compounding_requires_address() {
    let dir = tempfile::tempdir().unwrap();
    let mut req = request(dir.path());
    req.compounding = true;
    assert!(matches!(generate_keys(&req), Err(Error::MissingAddress)));
}

#[test]
fn test_invalid_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let mut req = request(dir.path());
    req.count = 0;
    assert!(matches!(generate_keys(&req), Err(Error::InvalidInput(_))));

    let mut req = request(dir.path());
    req.keystore_password = "short".to_string().into();
    assert!(matches!(generate_keys(&req), Err(Error::PasswordTooShort)));

    let mut req = request(dir.path());
    req.mnemonic = "abandon ".repeat(12).into();
    assert!(matches!(generate_keys(&req), Err(Error::InvalidMnemonic)));

    let mut req = request(dir.path());
    req.amount_gwei = GWEI_PER_ETH - 1;
    assert!(matches!(generate_keys(&req), Err(Error::InvalidAmount(_))));
    assert!(
        keystore_files(dir.path()).is_empty(),
        "no files written on error"
    );
}

#[test]
fn test_existing_mnemonic_bls_withdrawal_with_passphrase() {
    let dir_plain = tempfile::tempdir().unwrap();
    let plain = generate_keys(&request(dir_plain.path())).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let mut req = request(dir.path());
    req.mnemonic_password = "TREZOR".to_string().into();
    req.start_index = 2;
    req.count = 2;
    let out = generate_keys(&req).unwrap();
    check_output(&req, &out);
    assert!(
        !out.pubkeys.contains(&plain.pubkeys[0]),
        "passphrase changes the derived keys"
    );
}

#[test]
fn test_pbkdf2_matches_scrypt() {
    let dir_scrypt = tempfile::tempdir().unwrap();
    let scrypt = generate_keys(&request(dir_scrypt.path())).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let mut req = request(dir.path());
    req.kdf = KdfChoice::Pbkdf2;
    let pbkdf2 = generate_keys(&req).unwrap();
    check_output(&req, &pbkdf2);

    let a = read_deposits(&scrypt.deposit_data_file);
    let b = read_deposits(&pbkdf2.deposit_data_file);
    assert_eq!(a, b);
    let ks_json: Value = read_json(&pbkdf2.keystore_files[0]);
    assert_eq!(ks_json["crypto"]["kdf"]["function"], "pbkdf2");
    let ks_json: Value = read_json(&scrypt.keystore_files[0]);
    assert_eq!(ks_json["crypto"]["kdf"]["function"], "scrypt");
}

#[test]
fn test_existing_mnemonic_multiple_languages() {
    let ambiguous = "的 的 的 的 的 的 的 的 的 的 的 在";
    let dir = tempfile::tempdir().unwrap();
    let mut req = request(dir.path());
    req.mnemonic = ambiguous.to_string().into();
    assert!(matches!(
        generate_keys(&req),
        Err(Error::AmbiguousMnemonicLanguage(_))
    ));
    req.mnemonic_language = Some(Language::SimplifiedChinese);
    let out = generate_keys(&req).unwrap();
    check_output(&req, &out);
}

#[test]
fn test_regeneration() {
    let mnemonic = "legal winner thank year wave sausage worth useful legal winner thank yellow";
    let dir1 = tempfile::tempdir().unwrap();
    let mut req1 = request(dir1.path());
    req1.mnemonic = mnemonic.to_string().into();
    req1.count = 2;
    let out1 = generate_keys(&req1).unwrap();
    check_output(&req1, &out1);

    let dir2 = tempfile::tempdir().unwrap();
    let mut req2 = request(dir2.path());
    req2.mnemonic = mnemonic.to_string().into();
    req2.start_index = 1;
    req2.count = 2;
    let out2 = generate_keys(&req2).unwrap();
    check_output(&req2, &out2);

    let ks_1_1 = Keystore::from_json_file(&out1.keystore_files[1]).unwrap();
    let ks_2_0 = Keystore::from_json_file(&out2.keystore_files[0]).unwrap();
    assert!(out1.keystore_files[1].to_str().unwrap().contains("1_0_0"));
    assert!(out2.keystore_files[0].to_str().unwrap().contains("1_0_0"));
    assert_eq!(ks_1_1.pubkey(), ks_2_0.pubkey());
    assert_eq!(ks_1_1.path(), ks_2_0.path());
    assert_ne!(ks_1_1.uuid(), ks_2_0.uuid());
}

/// Parity with the Python proxy: identical deposit data, and keystores that decrypt to the same
/// keys, for every golden case.
#[test]
fn golden_parity() {
    for case in golden_cases("generate_keys") {
        let name = case.file_name().unwrap().to_str().unwrap().to_string();
        let params = read_json(&case.join("params.json"));
        let dir = tempfile::tempdir().unwrap();
        let req = request_from_params(&params, dir.path());
        let out = generate_keys(&req).unwrap_or_else(|e| panic!("{name}: {e}"));
        check_output(&req, &out);

        let expected: Value = read_json(&case.join("deposit_data.json"));
        let actual: Value = read_json(&out.deposit_data_file);
        assert_eq!(actual, expected, "{name}: deposit_data.json differs");

        for (i, file) in out.keystore_files.iter().enumerate() {
            let index = req.start_index + i as u32;
            let golden =
                Keystore::from_json_file(golden_dir(&name).join(format!("keystore-{index}.json")))
                    .unwrap();
            let ours = Keystore::from_json_file(file).unwrap();
            assert_eq!(ours.pubkey(), golden.pubkey(), "{name} keystore {index}");
            assert_eq!(ours.path(), golden.path(), "{name} keystore {index}");
            let golden_kp = golden.decrypt_keypair(PASSWORD.as_bytes()).unwrap();
            let our_kp = ours.decrypt_keypair(PASSWORD.as_bytes()).unwrap();
            assert_eq!(
                our_kp.sk.serialize().as_bytes(),
                golden_kp.sk.serialize().as_bytes(),
                "{name} keystore {index}"
            );
        }
    }
}
