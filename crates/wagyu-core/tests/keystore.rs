//! Port of `tests/test_key_handling/test_keystore.py` against the EIP-2335 test vectors.

mod common;

use common::{read_vector, TEST_VECTOR_PASSWORD, TEST_VECTOR_SECRET};
use eth2_keystore::{keypair_from_secret, Keystore};
use wagyu_core::keystore::{decrypt_keystore, encrypt_keystore, KdfChoice, KeystoreFile};

const VECTORS: [&str; 2] = ["keystore_pbkdf2.json", "keystore_scrypt.json"];

#[test]
fn test_json_serialization() {
    for name in VECTORS {
        let original = read_vector(name);
        let keystore = Keystore::from_json_str(&original.to_string()).unwrap();
        let file = KeystoreFile::from_keystore(&keystore).unwrap();
        let round_tripped: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&file).unwrap()).unwrap();
        assert_eq!(round_tripped, original, "{name}");
    }
}

#[test]
fn test_decrypt_test_vectors() {
    let expected = hex::decode(TEST_VECTOR_SECRET).unwrap();
    for name in VECTORS {
        let keystore = Keystore::from_json_str(&read_vector(name).to_string()).unwrap();
        let keypair = decrypt_keystore(&keystore, TEST_VECTOR_PASSWORD.as_bytes()).unwrap();
        assert_eq!(
            keypair.sk.serialize().as_bytes(),
            expected.as_slice(),
            "{name}"
        );
        assert_eq!(
            keypair.pk.as_hex_string()[2..],
            *keystore.pubkey(),
            "{name}"
        );
    }
}

#[test]
fn test_encrypt_decrypt_pbkdf2_random_iv() {
    let keypair = keypair_from_secret(&hex::decode(TEST_VECTOR_SECRET).unwrap()).unwrap();
    let keystore = encrypt_keystore(
        &keypair,
        TEST_VECTOR_PASSWORD.as_bytes(),
        "m/12381/3600/0/0/0".into(),
        KdfChoice::Pbkdf2,
    )
    .unwrap();
    assert!(matches!(
        keystore.kdf(),
        eth2_keystore::json_keystore::Kdf::Pbkdf2(_)
    ));
    let decrypted = decrypt_keystore(&keystore, TEST_VECTOR_PASSWORD.as_bytes()).unwrap();
    assert_eq!(
        decrypted.sk.serialize().as_bytes(),
        keypair.sk.serialize().as_bytes()
    );
}

#[test]
fn test_encrypt_decrypt_scrypt_random_iv() {
    let keypair = keypair_from_secret(&hex::decode(TEST_VECTOR_SECRET).unwrap()).unwrap();
    let keystore = encrypt_keystore(
        &keypair,
        TEST_VECTOR_PASSWORD.as_bytes(),
        "m/12381/3600/0/0/0".into(),
        KdfChoice::Scrypt,
    )
    .unwrap();
    let decrypted = decrypt_keystore(&keystore, TEST_VECTOR_PASSWORD.as_bytes()).unwrap();
    assert_eq!(
        decrypted.sk.serialize().as_bytes(),
        keypair.sk.serialize().as_bytes()
    );
    // Two encryptions of the same key differ (random salt, iv, uuid) but both decrypt.
    let again = encrypt_keystore(
        &keypair,
        TEST_VECTOR_PASSWORD.as_bytes(),
        "m/12381/3600/0/0/0".into(),
        KdfChoice::Scrypt,
    )
    .unwrap();
    assert_ne!(keystore.uuid(), again.uuid());
    assert_ne!(
        keystore.to_json_string().unwrap(),
        again.to_json_string().unwrap()
    );
}

#[test]
fn test_encrypt_decrypt_incorrect_password() {
    let keypair = keypair_from_secret(&hex::decode(TEST_VECTOR_SECRET).unwrap()).unwrap();
    let keystore = encrypt_keystore(
        &keypair,
        TEST_VECTOR_PASSWORD.as_bytes(),
        String::new(),
        KdfChoice::Scrypt,
    )
    .unwrap();
    let wrong = format!("{TEST_VECTOR_PASSWORD}incorrect");
    assert!(decrypt_keystore(&keystore, wrong.as_bytes()).is_err());
}

/// `_process_password`: passwords are NFKD-normalised and control characters are removed, so
/// `"a\x08c"` and `"ac"` are the same password.
#[test]
fn test_process_password() {
    let keypair = keypair_from_secret(&hex::decode(TEST_VECTOR_SECRET).unwrap()).unwrap();
    let keystore = encrypt_keystore(
        &keypair,
        "a\u{8}c".as_bytes(),
        String::new(),
        KdfChoice::Pbkdf2,
    )
    .unwrap();
    assert!(decrypt_keystore(&keystore, b"ac").is_ok());
    assert!(decrypt_keystore(&keystore, b"a\tc").is_ok());
    assert!(decrypt_keystore(&keystore, b"abc").is_err());
}
