//! IPC commands exposed to the React frontend. Each one maps onto `wagyu-core`; errors are
//! returned as their display text, which the UI shows verbatim.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use wagyu_core::btec::{
    GenerateBtecKeystoreRequest, GenerateBtecRequest, ValidateBlsCredentialsRequest,
};
use wagyu_core::deposit::{GenerateKeysOutput, GenerateKeysRequest};
use wagyu_core::exit::{GenerateExitFromKeystoreRequest, GenerateExitFromMnemonicRequest};
use wagyu_core::keystore::KdfChoice;
use wagyu_core::mnemonic::{language_from_name, Language};
use wagyu_core::partial_deposit::PartialDepositRequest;
use wagyu_core::validation::{
    deposit_amount_to_gwei, parse_bls_withdrawal_credentials_list, parse_validator_indices,
};
use wagyu_core::Network;

fn text(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn language(name: Option<String>) -> Result<Option<Language>, String> {
    name.filter(|n| !n.trim().is_empty())
        .map(|n| language_from_name(&n).map_err(text))
        .transpose()
}

/// Runs a CPU-heavy job off the main thread so the window stays responsive.
async fn blocking<T, F>(job: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> wagyu_core::Result<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(job)
        .await
        .map_err(text)?
        .map_err(text)
}

#[tauri::command]
pub fn create_mnemonic(language: Option<String>) -> Result<String, String> {
    let language = match language {
        Some(name) if !name.trim().is_empty() => language_from_name(&name).map_err(text)?,
        _ => Language::English,
    };
    wagyu_core::create_mnemonic(language)
        .map(|m| m.to_string())
        .map_err(text)
}

/// Validates a (possibly abbreviated) mnemonic and returns its full-word form.
#[tauri::command]
pub fn validate_mnemonic(mnemonic: String, language: Option<String>) -> Result<String, String> {
    let language = self::language(language)?;
    wagyu_core::reconstruct_mnemonic(&mnemonic, language)
        .map(|m| m.to_string())
        .map_err(text)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateKeysArgs {
    pub mnemonic: String,
    /// Index of the first validator to generate.
    pub index: u32,
    /// Deposit per validator as typed by the user, in ETH (or GNO), e.g. `"32"` or `"1.5"`.
    pub amount: String,
    pub count: u32,
    pub network: String,
    pub password: String,
    /// Empty string for BLS (`0x00`) withdrawal credentials.
    #[serde(default)]
    pub withdrawal_address: String,
    #[serde(default)]
    pub compounding: bool,
    pub folder: String,
    #[serde(default)]
    pub kdf: KdfChoice,
    #[serde(default)]
    pub mnemonic_language: Option<String>,
}

#[tauri::command]
pub async fn generate_keys(request: GenerateKeysArgs) -> Result<GenerateKeysOutput, String> {
    let mnemonic_language = language(request.mnemonic_language.clone())?;
    let network = Network::from_name(&request.network).map_err(text)?;
    let amount_gwei = deposit_amount_to_gwei(&request.amount, network.setting()).map_err(text)?;
    blocking(move || {
        let req = GenerateKeysRequest {
            mnemonic: request.mnemonic.into(),
            mnemonic_password: String::new().into(),
            mnemonic_language,
            start_index: request.index,
            count: request.count,
            amount_gwei,
            network,
            keystore_password: request.password.into(),
            withdrawal_address: Some(request.withdrawal_address),
            compounding: request.compounding,
            folder: PathBuf::from(request.folder),
            kdf: request.kdf,
        };
        wagyu_core::generate_keys(&req)
    })
    .await
}

#[tauri::command]
pub async fn validate_bls_credentials(
    chain: String,
    mnemonic: String,
    index: u32,
    withdrawal_credentials: String,
    mnemonic_language: Option<String>,
) -> Result<(), String> {
    let mnemonic_language = language(mnemonic_language)?;
    let network = Network::from_name(&chain).map_err(text)?;
    let credentials =
        parse_bls_withdrawal_credentials_list(&withdrawal_credentials).map_err(text)?;
    blocking(move || {
        wagyu_core::validate_bls_credentials(&ValidateBlsCredentialsRequest {
            network,
            mnemonic: mnemonic.into(),
            mnemonic_password: String::new().into(),
            mnemonic_language,
            start_index: index,
            bls_withdrawal_credentials: credentials,
        })
    })
    .await
}

/// Writes the BLS-to-execution-change file and returns its path.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn generate_bls_change(
    folder: String,
    chain: String,
    mnemonic: String,
    index: u32,
    indices: String,
    withdrawal_credentials: String,
    execution_address: String,
    mnemonic_language: Option<String>,
) -> Result<String, String> {
    let mnemonic_language = language(mnemonic_language)?;
    let network = Network::from_name(&chain).map_err(text)?;
    let validator_indices = parse_validator_indices(&indices).map_err(text)?;
    let credentials =
        parse_bls_withdrawal_credentials_list(&withdrawal_credentials).map_err(text)?;
    blocking(move || {
        wagyu_core::generate_bls_to_execution_change(&GenerateBtecRequest {
            folder: PathBuf::from(folder),
            network,
            mnemonic: mnemonic.into(),
            mnemonic_password: String::new().into(),
            mnemonic_language,
            start_index: index,
            validator_indices,
            bls_withdrawal_credentials: credentials,
            withdrawal_address: execution_address,
        })
        .map(|p| p.to_string_lossy().into_owned())
    })
    .await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitTransactionsArgs {
    pub folder: String,
    pub chain: String,
    pub mnemonic: String,
    /// EIP-2334 index of the first validator.
    pub index: u32,
    /// Comma separated beacon-chain validator indices.
    pub indices: String,
    #[serde(default)]
    pub epoch: u64,
    #[serde(default)]
    pub mnemonic_language: Option<String>,
}

/// Writes one signed exit per validator and returns the file paths.
#[tauri::command]
pub async fn generate_exit_transactions(
    request: ExitTransactionsArgs,
) -> Result<Vec<String>, String> {
    let mnemonic_language = language(request.mnemonic_language.clone())?;
    let network = Network::from_name(&request.chain).map_err(text)?;
    let validator_indices = parse_validator_indices(&request.indices).map_err(text)?;
    blocking(move || {
        wagyu_core::generate_exit_transactions(&GenerateExitFromMnemonicRequest {
            folder: PathBuf::from(request.folder),
            network,
            mnemonic: request.mnemonic.into(),
            mnemonic_password: String::new().into(),
            mnemonic_language,
            start_index: request.index,
            validator_indices,
            epoch: request.epoch,
        })
        .map(|paths| {
            paths
                .into_iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect()
        })
    })
    .await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitTransactionKeystoreArgs {
    pub folder: String,
    pub chain: String,
    pub keystore: String,
    pub keystore_password: String,
    pub validator_index: u64,
    #[serde(default)]
    pub epoch: u64,
}

/// Signs an exit with a keystore file and returns the path of the written file.
#[tauri::command]
pub async fn generate_exit_transaction_keystore(
    request: ExitTransactionKeystoreArgs,
) -> Result<String, String> {
    let network = Network::from_name(&request.chain).map_err(text)?;
    blocking(move || {
        wagyu_core::generate_exit_transaction_from_keystore(&GenerateExitFromKeystoreRequest {
            folder: PathBuf::from(request.folder),
            network,
            keystore_path: PathBuf::from(request.keystore),
            keystore_password: request.keystore_password.into(),
            validator_index: request.validator_index,
            epoch: request.epoch,
        })
        .map(|p| p.to_string_lossy().into_owned())
    })
    .await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartialDepositArgs {
    pub folder: String,
    pub chain: String,
    pub keystore: String,
    pub keystore_password: String,
    /// Deposit as typed by the user, in ETH (or GNO), e.g. `"32"` or `"1.5"`.
    pub amount: String,
    pub withdrawal_address: String,
    #[serde(default)]
    pub compounding: bool,
}

/// Writes a single-entry deposit data file for topping up an existing validator and returns
/// its path.
#[tauri::command]
pub async fn generate_partial_deposit(request: PartialDepositArgs) -> Result<String, String> {
    let network = Network::from_name(&request.chain).map_err(text)?;
    let amount_gwei = deposit_amount_to_gwei(&request.amount, network.setting()).map_err(text)?;
    blocking(move || {
        wagyu_core::generate_partial_deposit(&PartialDepositRequest {
            folder: PathBuf::from(request.folder),
            network,
            keystore_path: PathBuf::from(request.keystore),
            keystore_password: request.keystore_password.into(),
            amount_gwei,
            withdrawal_address: request.withdrawal_address,
            compounding: request.compounding,
        })
        .map(|p| p.to_string_lossy().into_owned())
    })
    .await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlsChangeKeystoreArgs {
    pub folder: String,
    pub chain: String,
    pub keystore: String,
    pub keystore_password: String,
    pub validator_index: u64,
    pub withdrawal_address: String,
}

/// Writes a BLS-to-execution change signed with a keystore and returns the file path.
#[tauri::command]
pub async fn generate_bls_change_keystore(
    request: BlsChangeKeystoreArgs,
) -> Result<String, String> {
    let network = Network::from_name(&request.chain).map_err(text)?;
    blocking(move || {
        wagyu_core::generate_bls_to_execution_change_keystore(&GenerateBtecKeystoreRequest {
            folder: PathBuf::from(request.folder),
            network,
            keystore_path: PathBuf::from(request.keystore),
            keystore_password: request.keystore_password.into(),
            validator_index: request.validator_index,
            withdrawal_address: request.withdrawal_address,
        })
        .map(|p| p.to_string_lossy().into_owned())
    })
    .await
}

/// The hex public key in a keystore file, so the UI can confirm which validator it belongs to.
#[tauri::command]
pub fn keystore_pubkey(path: String) -> Result<String, String> {
    wagyu_core::keystore::keystore_file_pubkey(Path::new(&path)).map_err(text)
}

#[tauri::command]
pub fn is_address(address: String) -> bool {
    wagyu_core::validation::is_address(&address)
}

#[tauri::command]
pub fn does_directory_exist(directory: String) -> bool {
    wagyu_core::fs::does_directory_exist(Path::new(&directory))
}

#[tauri::command]
pub fn is_directory_writable(directory: String) -> bool {
    wagyu_core::fs::is_directory_writable(Path::new(&directory))
}

/// Path of the first file in `directory` whose name starts with `starts_with`, or `""`.
#[tauri::command]
pub fn find_first_file(directory: String, starts_with: String) -> Result<String, String> {
    wagyu_core::fs::find_first_file(Path::new(&directory), &starts_with)
        .map(|p| {
            p.map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
        .map_err(text)
}

#[derive(Debug, Serialize)]
pub struct VersionInfo {
    pub version: String,
    pub commit: String,
}

#[tauri::command]
pub fn version_info<R: tauri::Runtime>(app: AppHandle<R>) -> VersionInfo {
    VersionInfo {
        version: app.package_info().version.to_string(),
        commit: crate::COMMIT_HASH.to_string(),
    }
}

#[tauri::command]
pub fn quit<R: tauri::Runtime>(app: AppHandle<R>) {
    app.exit(0);
}
