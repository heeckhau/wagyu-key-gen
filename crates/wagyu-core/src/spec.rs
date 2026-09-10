//! Consensus-spec SSZ containers and signing domains, ported from `ethstaker_deposit/utils/ssz.py`.
//!
//! The field types deliberately mirror Lighthouse's `types` crate so that the `TreeHash`
//! implementations from `bls` and `tree_hash` are reused unchanged. Nothing here performs
//! cryptography beyond Merkleization via `tree_hash`.

use alloy_primitives::Address;
use bls::{Hash256, PublicKeyBytes, SignatureBytes};
use tree_hash::TreeHash;
use tree_hash_derive::TreeHash;

/// `DOMAIN_DEPOSIT` (little-endian `0x03000000`).
pub const DOMAIN_DEPOSIT: u32 = 3;
/// `DOMAIN_VOLUNTARY_EXIT` (`0x04000000`).
pub const DOMAIN_VOLUNTARY_EXIT: u32 = 4;
/// `DOMAIN_BLS_TO_EXECUTION_CHANGE` (`0x0A000000`).
pub const DOMAIN_BLS_TO_EXECUTION_CHANGE: u32 = 10;

/// <https://github.com/ethereum/consensus-specs/blob/dev/specs/phase0/beacon-chain.md#depositmessage>
#[derive(Debug, Clone, PartialEq, TreeHash)]
pub struct DepositMessage {
    pub pubkey: PublicKeyBytes,
    pub withdrawal_credentials: Hash256,
    pub amount: u64,
}

/// <https://github.com/ethereum/consensus-specs/blob/dev/specs/phase0/beacon-chain.md#depositdata>
#[derive(Debug, Clone, PartialEq, TreeHash)]
pub struct DepositData {
    pub pubkey: PublicKeyBytes,
    pub withdrawal_credentials: Hash256,
    pub amount: u64,
    pub signature: SignatureBytes,
}

/// <https://github.com/ethereum/consensus-specs/blob/dev/specs/capella/beacon-chain.md#blstoexecutionchange>
#[derive(Debug, Clone, PartialEq, TreeHash)]
pub struct BlsToExecutionChange {
    pub validator_index: u64,
    pub from_bls_pubkey: PublicKeyBytes,
    pub to_execution_address: Address,
}

/// <https://github.com/ethereum/consensus-specs/blob/dev/specs/capella/beacon-chain.md#signedblstoexecutionchange>
#[derive(Debug, Clone, PartialEq, TreeHash)]
pub struct SignedBlsToExecutionChange {
    pub message: BlsToExecutionChange,
    pub signature: SignatureBytes,
}

/// <https://github.com/ethereum/consensus-specs/blob/dev/specs/phase0/beacon-chain.md#voluntaryexit>
#[derive(Debug, Clone, PartialEq, TreeHash)]
pub struct VoluntaryExit {
    pub epoch: u64,
    pub validator_index: u64,
}

/// <https://github.com/ethereum/consensus-specs/blob/dev/specs/phase0/beacon-chain.md#signingdata>
#[derive(Debug, Clone, PartialEq, TreeHash)]
pub struct SigningData {
    pub object_root: Hash256,
    pub domain: Hash256,
}

/// <https://github.com/ethereum/consensus-specs/blob/dev/specs/phase0/beacon-chain.md#forkdata>
#[derive(Debug, Clone, PartialEq, TreeHash)]
pub struct ForkData {
    pub current_version: [u8; 4],
    pub genesis_validators_root: Hash256,
}

/// `compute_fork_data_root` from the spec.
pub fn compute_fork_data_root(
    current_version: [u8; 4],
    genesis_validators_root: Hash256,
) -> Hash256 {
    ForkData {
        current_version,
        genesis_validators_root,
    }
    .tree_hash_root()
}

/// `compute_domain` from the spec: `domain_type ‖ fork_data_root[:28]`.
pub fn compute_domain(
    domain_type: u32,
    fork_version: [u8; 4],
    genesis_validators_root: Hash256,
) -> Hash256 {
    let mut domain = [0u8; 32];
    domain[..4].copy_from_slice(&domain_type.to_le_bytes());
    domain[4..].copy_from_slice(
        &compute_fork_data_root(fork_version, genesis_validators_root).as_slice()[..28],
    );
    Hash256::new(domain)
}

/// Deposit domain: the genesis validators root is always zero for deposits.
pub fn compute_deposit_domain(fork_version: [u8; 4]) -> Hash256 {
    compute_domain(DOMAIN_DEPOSIT, fork_version, Hash256::ZERO)
}

/// Fork data root used by the deposit domain (zero genesis validators root).
pub fn compute_deposit_fork_data_root(current_version: [u8; 4]) -> Hash256 {
    compute_fork_data_root(current_version, Hash256::ZERO)
}

pub fn compute_bls_to_execution_change_domain(
    fork_version: [u8; 4],
    genesis_validators_root: Hash256,
) -> Hash256 {
    compute_domain(
        DOMAIN_BLS_TO_EXECUTION_CHANGE,
        fork_version,
        genesis_validators_root,
    )
}

pub fn compute_voluntary_exit_domain(
    fork_version: [u8; 4],
    genesis_validators_root: Hash256,
) -> Hash256 {
    compute_domain(DOMAIN_VOLUNTARY_EXIT, fork_version, genesis_validators_root)
}

/// `compute_signing_root` from the spec: the hash tree root of `SigningData`.
pub fn compute_signing_root<T: TreeHash>(object: &T, domain: Hash256) -> Hash256 {
    SigningData {
        object_root: object.tree_hash_root(),
        domain,
    }
    .tree_hash_root()
}
