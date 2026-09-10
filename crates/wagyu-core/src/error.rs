use std::path::PathBuf;

use thiserror::Error;

/// All errors surfaced by `wagyu-core`.
///
/// The `Display` text is what the UI shows to the user, so keep it human readable.
#[derive(Debug, Error)]
pub enum Error {
    /// The mnemonic has a bad word count, an unknown word or a bad checksum.
    ///
    /// The text intentionally matches the message of the previous Python implementation, which
    /// the UI used to string-match.
    #[error("That is not a valid mnemonic, please check for typos.")]
    InvalidMnemonic,

    /// The mnemonic validates in more than one language and no language was specified.
    #[error("Multiple valid languages found: {0}. Please specify the mnemonic language.")]
    AmbiguousMnemonicLanguage(String),

    #[error("Unsupported mnemonic language: {0}")]
    UnsupportedLanguage(String),

    #[error("Unsupported network: {0}")]
    UnsupportedNetwork(String),

    #[error("The given withdrawal address is not a valid Ethereum address.")]
    InvalidAddress,

    /// A withdrawal address is required for this operation (e.g. compounding credentials).
    #[error("A withdrawal address is required.")]
    MissingAddress,

    #[error("{0}")]
    InvalidAmount(String),

    #[error("Password must be at least 12 characters.")]
    PasswordTooShort,

    #[error("{0}")]
    InvalidInput(String),

    #[error(
        "The number of validator indices must match the number of BLS withdrawal credentials."
    )]
    IndicesCredentialsMismatch,

    #[error(
        "Those BLS credentials do not match those we can derive from your Secret Recovery Phrase."
    )]
    BlsCredentialsMismatch,

    #[error("This network has no genesis validators root, so a BLS to execution change cannot be signed for it.")]
    MissingGenesisValidatorsRoot,

    #[error("Failed to verify the keystores.")]
    KeystoreVerification,

    #[error("Failed to verify the deposit data JSON file.")]
    DepositVerification,

    #[error("Failed to verify the BLS to execution change JSON file.")]
    BtecVerification,

    #[error("Failed to verify the exit transaction JSON file.")]
    ExitVerification,

    #[error("The keystore password is incorrect.")]
    WrongKeystorePassword,

    #[error("{0} is not a valid keystore file.")]
    InvalidKeystoreFile(PathBuf),

    #[error("Keystore error: {0:?}")]
    Keystore(eth2_keystore::Error),

    #[error("Key derivation error: {0:?}")]
    KeyDerivation(eth2_wallet::Error),

    #[error("BLS error: {0:?}")]
    Bls(bls::Error),

    #[error("{context}: {path}: {source}")]
    Io {
        context: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Random number generation failed: {0}")]
    Rng(String),
}

impl Error {
    pub(crate) fn io(
        context: &'static str,
        path: impl Into<PathBuf>,
        source: std::io::Error,
    ) -> Self {
        Error::Io {
            context,
            path: path.into(),
            source,
        }
    }
}

impl From<eth2_keystore::Error> for Error {
    fn from(e: eth2_keystore::Error) -> Self {
        Error::Keystore(e)
    }
}

impl From<eth2_wallet::Error> for Error {
    fn from(e: eth2_wallet::Error) -> Self {
        Error::KeyDerivation(e)
    }
}

impl From<bls::Error> for Error {
    fn from(e: bls::Error) -> Self {
        Error::Bls(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
