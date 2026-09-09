//! Port of `tests/test_utils/test_ssz.py`.

use bls::{Hash256, PublicKeyBytes};
use wagyu_core::spec::{
    compute_deposit_domain, compute_deposit_fork_data_root, compute_signing_root, DepositMessage,
};

fn h(hex_str: &str) -> Hash256 {
    Hash256::from_slice(&hex::decode(hex_str).unwrap())
}

#[test]
fn test_compute_deposit_domain() {
    assert_eq!(
        compute_deposit_domain([0x12; 4]),
        h("030000000d66608af557f4fadbfce248ac37f6e7639ce371100c43d15aad05cb")
    );
}

#[test]
fn test_compute_deposit_fork_data_root() {
    assert_eq!(
        compute_deposit_fork_data_root([0x12; 4]),
        h("0d66608af557f4fadbfce248ac37f6e7639ce371100c43d15aad05cb08ac1dc2")
    );
}

#[test]
fn test_compute_signing_root() {
    let message = DepositMessage {
        pubkey: PublicKeyBytes::deserialize(&[0x12; 48]).unwrap(),
        withdrawal_credentials: Hash256::repeat_byte(0x12),
        amount: 100,
    };
    assert_eq!(
        compute_signing_root(&message, Hash256::repeat_byte(0x12)),
        h("67a3330ff87bdb46bb7b80ca7a641e398d6ac4e87a6856527cacc829fb61896f")
    );
}
