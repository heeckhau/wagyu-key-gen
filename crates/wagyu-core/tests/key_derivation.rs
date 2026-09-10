//! Port of `test_key_derivation/test_tree.py` and `test_path.py` (the parts that test observable
//! EIP-2333 behaviour; Lamport intermediates are internal to `eth2_key_derivation`).

mod common;

use common::{big_int_to_32_bytes, hex_to_bytes, read_vector};
use eth2_key_derivation::DerivedKey;
use eth2_wallet::{KeyType, ValidatorPath};
use wagyu_core::chain::MAINNET;
use wagyu_core::credential::Credential;
use wagyu_core::mnemonic::parse_mnemonic;

#[test]
fn test_derive_master_sk() {
    let vectors = read_vector("tree_kdf.json");
    let tests = vectors["kdf_tests"].as_array().unwrap();
    assert!(tests.len() >= 4);
    for test in tests {
        let seed = hex_to_bytes(&test["seed"]);
        let master = DerivedKey::from_seed(&seed).unwrap();
        assert_eq!(master.secret(), big_int_to_32_bytes(&test["master_SK"]));
    }
    // Python rejects seeds shorter than 32 bytes; Lighthouse rejects the empty seed.
    assert!(DerivedKey::from_seed(&[]).is_err());
}

#[test]
fn test_derive_child_sk_valid() {
    let vectors = read_vector("tree_kdf.json");
    for test in vectors["kdf_tests"].as_array().unwrap() {
        let seed = hex_to_bytes(&test["seed"]);
        let index = test["child_index"].as_u64().unwrap() as u32;
        let child = DerivedKey::from_seed(&seed).unwrap().child(index);
        assert_eq!(child.secret(), big_int_to_32_bytes(&test["child_SK"]));
    }
}

#[test]
fn test_mnemonic_and_path_to_key() {
    let v = read_vector("tree_kdf_intermediate.json");
    let mnemonic = parse_mnemonic(v["mnemonic"].as_str().unwrap(), None).unwrap();
    let seed = mnemonic.seed(v["password"].as_str().unwrap());
    assert_eq!(seed.as_slice(), hex_to_bytes(&v["seed"]).as_slice());
    let master = DerivedKey::from_seed(seed.as_ref()).unwrap();
    assert_eq!(master.secret(), big_int_to_32_bytes(&v["master_SK"]));
    assert_eq!(v["path"], "m/0");
    let child = master.child(v["child_index"].as_u64().unwrap() as u32);
    assert_eq!(child.secret(), big_int_to_32_bytes(&v["child_SK"]));
}

#[test]
fn test_validator_paths() {
    assert_eq!(
        ValidatorPath::new(0, KeyType::Voting).to_string(),
        "m/12381/3600/0/0/0"
    );
    assert_eq!(
        ValidatorPath::new(0, KeyType::Withdrawal).to_string(),
        "m/12381/3600/0/0"
    );
    assert_eq!(
        ValidatorPath::new(3141592653, KeyType::Voting).to_string(),
        "m/12381/3600/3141592653/0/0"
    );
    let seed = [0x12u8; 64];
    let credential = Credential::new(&seed, 7, 32_000_000_000, &MAINNET, None, false).unwrap();
    assert_eq!(credential.signing_key_path(), "m/12381/3600/7/0/0");
}
