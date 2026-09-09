//! EIP-2335 keystores. Encryption and decryption are entirely `eth2_keystore` (Lighthouse); this
//! module only chooses the KDF parameters the deposit-cli uses and writes files in its layout.

use std::path::Path;

use bls::Keypair;
use eth2_keystore::json_keystore::{Crypto, JsonKeystore, Kdf, Pbkdf2, Prf};
use eth2_keystore::{Keystore, KeystoreBuilder, DKLEN, SALT_SIZE};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::fs::write_sensitive_json;

/// Which key derivation function protects the keystore. `Scrypt` is the deposit-cli default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KdfChoice {
    #[default]
    Scrypt,
    Pbkdf2,
}

/// PBKDF2 iteration count used by the deposit-cli (`2**18`).
pub const PBKDF2_ROUNDS: u32 = 262_144;

/// Encrypts `keypair` under `password`. Scrypt uses `n=2^18, r=8, p=1, dklen=32`
/// (`eth2_keystore::default_kdf`); PBKDF2 uses `c=2^18, prf=hmac-sha256, dklen=32`.
pub fn encrypt_keystore(
    keypair: &Keypair,
    password: &[u8],
    path: String,
    kdf: KdfChoice,
) -> Result<Keystore> {
    let builder = KeystoreBuilder::new(keypair, password, path)?;
    let builder = match kdf {
        KdfChoice::Scrypt => builder,
        KdfChoice::Pbkdf2 => {
            let salt: [u8; SALT_SIZE] = rand::random();
            builder.kdf(Kdf::Pbkdf2(Pbkdf2 {
                c: PBKDF2_ROUNDS,
                dklen: DKLEN,
                prf: Prf::HmacSha256,
                salt: salt.to_vec().into(),
            }))
        }
    };
    Ok(builder.build()?)
}

/// Decrypts a keystore, verifying that the recovered public key matches the file.
pub fn decrypt_keystore(keystore: &Keystore, password: &[u8]) -> Result<Keypair> {
    Ok(keystore.decrypt_keypair(password)?)
}

/// The deposit-cli file name: `keystore-m_12381_3600_<i>_0_0-<timestamp>.json`.
pub fn keystore_file_name(path: &str, timestamp: u64) -> String {
    format!("keystore-{}-{}.json", path.replace('/', "_"), timestamp)
}

/// The keystore JSON exactly as the deposit-cli lays it out: same fields, same order, no extra
/// (`name`) field. Parsers do not care about key order, but byte-for-byte parity with the
/// previous implementation keeps diffs of generated files meaningful.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeystoreFile {
    pub crypto: Crypto,
    pub description: String,
    pub pubkey: String,
    pub path: String,
    pub uuid: String,
    pub version: u8,
}

impl KeystoreFile {
    pub fn from_keystore(keystore: &Keystore) -> Result<Self> {
        let json: JsonKeystore = serde_json::from_value(serde_json::to_value(keystore)?)?;
        Ok(KeystoreFile {
            crypto: json.crypto,
            description: json.description.unwrap_or_default(),
            pubkey: json.pubkey,
            path: json.path.unwrap_or_default(),
            uuid: json.uuid.to_string(),
            version: 4,
        })
    }
}

/// Writes the keystore as a new read-only file in the deposit-cli layout.
pub fn write_keystore_file(path: &Path, keystore: &Keystore) -> Result<()> {
    write_sensitive_json(path, &KeystoreFile::from_keystore(keystore)?)
}

/// Reads a keystore file written by this crate, the deposit-cli or Lighthouse.
pub fn read_keystore_file(path: &Path) -> Result<Keystore> {
    Ok(Keystore::from_json_file(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eth2_keystore::keypair_from_secret;

    fn keypair() -> Keypair {
        keypair_from_secret(
            &hex::decode("000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f")
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn file_layout_matches_deposit_cli() {
        let ks = encrypt_keystore(
            &keypair(),
            b"MyPasswordIs",
            "m/12381/3600/0/0/0".into(),
            KdfChoice::Scrypt,
        )
        .unwrap();
        let file = KeystoreFile::from_keystore(&ks).unwrap();
        let json = serde_json::to_string(&file).unwrap();
        let keys: Vec<String> =
            serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&json)
                .unwrap()
                .keys()
                .cloned()
                .collect();
        let mut expected = vec!["crypto", "description", "pubkey", "path", "uuid", "version"];
        expected.sort();
        assert_eq!(keys, expected);
        assert!(!json.contains("\"name\""));
        assert!(json.starts_with(r#"{"crypto":{"kdf":{"function":"scrypt","params":{"dklen":32,"n":262144,"r":8,"p":1,"salt":""#));
        assert!(json.contains(r#""description":"","pubkey":"9612d7a727c9d0a22e185a1c768478dfe919cada9266988cb32359c11f2b7b27f4ae4040902382ae2910c15e2b420d07","path":"m/12381/3600/0/0/0","uuid":""#));
        assert!(json.ends_with(r#""version":4}"#));
        // And it reads back as a Lighthouse keystore.
        let parsed = Keystore::from_json_str(&json).unwrap();
        assert_eq!(parsed.path().as_deref(), Some("m/12381/3600/0/0/0"));
        assert_eq!(
            decrypt_keystore(&parsed, b"MyPasswordIs").unwrap().pk,
            keypair().pk
        );
    }

    #[test]
    fn pbkdf2_layout() {
        let ks = encrypt_keystore(
            &keypair(),
            b"MyPasswordIs",
            "m/12381/3600/1/0/0".into(),
            KdfChoice::Pbkdf2,
        )
        .unwrap();
        let json = serde_json::to_string(&KeystoreFile::from_keystore(&ks).unwrap()).unwrap();
        assert!(json.starts_with(r#"{"crypto":{"kdf":{"function":"pbkdf2","params":{"c":262144,"dklen":32,"prf":"hmac-sha256","salt":""#));
        assert_eq!(
            decrypt_keystore(&ks, b"MyPasswordIs").unwrap().pk,
            keypair().pk
        );
    }

    #[test]
    fn file_names() {
        assert_eq!(
            keystore_file_name("m/12381/3600/7/0/0", 1700000000),
            "keystore-m_12381_3600_7_0_0-1700000000.json"
        );
    }
}
