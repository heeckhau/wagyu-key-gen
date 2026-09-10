//! Chain settings, ported from `ethstaker_deposit/settings.py`.

use bls::Hash256;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// 1 ETH (or 1 GNO on Gnosis chains) expressed in gwei.
pub const GWEI_PER_ETH: u64 = 1_000_000_000;
/// The largest deposit the deposit contract accepts, in gwei (2048 ETH).
pub const MAX_DEPOSIT_AMOUNT_GWEI: u64 = 2048 * GWEI_PER_ETH;
/// The version string written into `deposit_data` and `bls_to_execution_change` files.
///
/// This is the file format version of ethstaker-deposit-cli whose output this crate reproduces
/// byte for byte. The Ethereum launchpad checks this field against a minimum version.
pub const DEPOSIT_CLI_VERSION: &str = "1.2.2";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Sepolia,
    Holesky,
    Hoodi,
    Ephemery,
    Gnosis,
    Chiado,
}

impl Network {
    pub const ALL: [Network; 7] = [
        Network::Mainnet,
        Network::Sepolia,
        Network::Holesky,
        Network::Hoodi,
        Network::Ephemery,
        Network::Gnosis,
        Network::Chiado,
    ];

    /// The lowercase network name used in files and on the CLI (`mainnet`, `hoodi`, ...).
    pub fn name(self) -> &'static str {
        self.setting().network_name
    }

    /// Case-insensitive lookup by name, so both `Mainnet` (UI) and `mainnet` (files) work.
    pub fn from_name(name: &str) -> Result<Network> {
        let lower = name.trim().to_ascii_lowercase();
        Network::ALL
            .iter()
            .copied()
            .find(|n| n.name() == lower)
            .ok_or_else(|| Error::UnsupportedNetwork(name.to_string()))
    }

    pub fn setting(self) -> &'static ChainSetting {
        match self {
            Network::Mainnet => &MAINNET,
            Network::Sepolia => &SEPOLIA,
            Network::Holesky => &HOLESKY,
            Network::Hoodi => &HOODI,
            Network::Ephemery => &EPHEMERY,
            Network::Gnosis => &GNOSIS,
            Network::Chiado => &CHIADO,
        }
    }
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Per-chain constants (`BaseChainSetting` in the Python code).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainSetting {
    pub network_name: &'static str,
    pub genesis_fork_version: [u8; 4],
    /// Capella fork version, used for voluntary exits (EIP-7044).
    pub exit_fork_version: [u8; 4],
    /// `None` for chains whose genesis changes regularly (Ephemery).
    pub genesis_validators_root: Option<Hash256>,
    /// Gnosis chains deposit GNO, which the deposit contract multiplies by 32.
    pub multiplier: u64,
    /// Balance needed to activate a validator, in the chain's own unit (ETH or GNO), in gwei.
    pub min_activation_amount_gwei: u64,
    /// Smallest deposit accepted by this CLI, in the chain's own unit, in gwei.
    pub min_deposit_amount_gwei: u64,
}

impl ChainSetting {
    /// Lower bound for `DepositMessage.amount` (already multiplied), in gwei.
    pub fn min_deposit_message_amount_gwei(&self) -> u64 {
        self.min_deposit_amount_gwei * self.multiplier
    }

    /// The default deposit for a fresh validator (32 ETH, 1 GNO), already multiplied, in gwei.
    pub fn default_deposit_amount_gwei(&self) -> u64 {
        self.min_activation_amount_gwei * self.multiplier
    }
}

const fn h256(bytes: [u8; 32]) -> Hash256 {
    Hash256::new(bytes)
}

pub static MAINNET: ChainSetting = ChainSetting {
    network_name: "mainnet",
    genesis_fork_version: [0x00, 0x00, 0x00, 0x00],
    exit_fork_version: [0x03, 0x00, 0x00, 0x00],
    genesis_validators_root: Some(h256(hex_literal(
        "4b363db94e286120d76eb905340fdd4e54bfe9f06bf33ff6cf5ad27f511bfe95",
    ))),
    multiplier: 1,
    min_activation_amount_gwei: 32 * GWEI_PER_ETH,
    min_deposit_amount_gwei: GWEI_PER_ETH,
};

pub static SEPOLIA: ChainSetting = ChainSetting {
    network_name: "sepolia",
    genesis_fork_version: [0x90, 0x00, 0x00, 0x69],
    exit_fork_version: [0x90, 0x00, 0x00, 0x72],
    genesis_validators_root: Some(h256(hex_literal(
        "d8ea171f3c94aea21ebc42a1ed61052acf3f9209c00e4efbaaddac09ed9b8078",
    ))),
    multiplier: 1,
    min_activation_amount_gwei: 32 * GWEI_PER_ETH,
    min_deposit_amount_gwei: GWEI_PER_ETH,
};

pub static HOLESKY: ChainSetting = ChainSetting {
    network_name: "holesky",
    genesis_fork_version: [0x01, 0x01, 0x70, 0x00],
    exit_fork_version: [0x04, 0x01, 0x70, 0x00],
    genesis_validators_root: Some(h256(hex_literal(
        "9143aa7c615a7f7115e2b6aac319c03529df8242ae705fba9df39b79c59fa8b1",
    ))),
    multiplier: 1,
    min_activation_amount_gwei: 32 * GWEI_PER_ETH,
    min_deposit_amount_gwei: GWEI_PER_ETH,
};

pub static HOODI: ChainSetting = ChainSetting {
    network_name: "hoodi",
    genesis_fork_version: [0x10, 0x00, 0x09, 0x10],
    exit_fork_version: [0x40, 0x00, 0x09, 0x10],
    genesis_validators_root: Some(h256(hex_literal(
        "212f13fc4df078b6cb7db228f1c8307566dcecf900867401a92023d7ba99cb5f",
    ))),
    multiplier: 1,
    min_activation_amount_gwei: 32 * GWEI_PER_ETH,
    min_deposit_amount_gwei: GWEI_PER_ETH,
};

pub static EPHEMERY: ChainSetting = ChainSetting {
    network_name: "ephemery",
    genesis_fork_version: [0x10, 0x00, 0x10, 0x1b],
    exit_fork_version: [0x40, 0x00, 0x10, 0x1b],
    genesis_validators_root: None,
    multiplier: 1,
    min_activation_amount_gwei: 32 * GWEI_PER_ETH,
    min_deposit_amount_gwei: GWEI_PER_ETH,
};

pub static GNOSIS: ChainSetting = ChainSetting {
    network_name: "gnosis",
    genesis_fork_version: [0x00, 0x00, 0x00, 0x64],
    exit_fork_version: [0x03, 0x00, 0x00, 0x64],
    genesis_validators_root: Some(h256(hex_literal(
        "f5dcb5564e829aab27264b9becd5dfaa017085611224cb3036f573368dbb9d47",
    ))),
    multiplier: 32,
    min_activation_amount_gwei: GWEI_PER_ETH,
    // 0.03125 GNO
    min_deposit_amount_gwei: GWEI_PER_ETH / 32,
};

pub static CHIADO: ChainSetting = ChainSetting {
    network_name: "chiado",
    genesis_fork_version: [0x00, 0x00, 0x00, 0x6f],
    exit_fork_version: [0x03, 0x00, 0x00, 0x6f],
    genesis_validators_root: Some(h256(hex_literal(
        "9d642dac73058fbf39c0ae41ab1e34e4d889043cb199851ded7095bc99eb4c1e",
    ))),
    multiplier: 32,
    min_activation_amount_gwei: GWEI_PER_ETH,
    min_deposit_amount_gwei: GWEI_PER_ETH / 32,
};

/// Decodes a 64-character hex string at compile time.
const fn hex_literal(s: &str) -> [u8; 32] {
    let bytes = s.as_bytes();
    assert!(bytes.len() == 64, "expected 64 hex characters");
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        out[i] = (hex_nibble(bytes[2 * i]) << 4) | hex_nibble(bytes[2 * i + 1]);
        i += 1;
    }
    out
}

const fn hex_nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => panic!("invalid hex character"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_names_round_trip() {
        for n in Network::ALL {
            assert_eq!(Network::from_name(n.name()).unwrap(), n);
            assert_eq!(Network::from_name(&n.name().to_uppercase()).unwrap(), n);
        }
        assert!(Network::from_name("prater").is_err());
    }

    #[test]
    fn gnosis_amounts() {
        assert_eq!(GNOSIS.min_deposit_message_amount_gwei(), GWEI_PER_ETH);
        assert_eq!(GNOSIS.default_deposit_amount_gwei(), 32 * GWEI_PER_ETH);
        assert_eq!(MAINNET.default_deposit_amount_gwei(), 32 * GWEI_PER_ETH);
    }
}
