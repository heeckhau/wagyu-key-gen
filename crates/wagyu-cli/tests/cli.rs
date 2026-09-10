//! End-to-end tests: run the built binary the way a user would, non-interactively, and compare
//! its output files with the ethstaker-deposit-cli golden fixtures of `wagyu-core`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const SISTER: &str = "sister protect peanut hill ready work profit fit wish want small inflict flip member tail between sick setup bright duck morning sell paper worry";
const CREDS: &str = "0x00bd0b5a34de5fb17df08410b5e615dda87caf4fb72d0aac91ce5e52fc6aa8de,0x00a75d83f169fa6923f3dd78386d9608fab710d8f7fcf71ba9985893675d5382";

fn golden(case: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../wagyu-core/tests/vectors/golden")
        .join(case)
}

fn mainnet_keystore() -> PathBuf {
    golden("keys_mainnet_bls_3").join("keystore-0.json")
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wagyu-cli"))
        .arg("--non_interactive")
        .args(args)
        .output()
        .expect("run wagyu-cli")
}

fn run_ok(args: &[&str]) -> String {
    let output = run(args);
    assert!(
        output.status.success(),
        "exit {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn run_err(args: &[&str]) -> String {
    let output = run(args);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8(output.stderr).unwrap()
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display())),
    )
    .unwrap()
}

/// The files in `dir` whose name starts with `prefix`, sorted.
fn files_with_prefix(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.file_name().unwrap().to_str().unwrap().starts_with(prefix))
        .collect();
    files.sort();
    files
}

#[test]
fn help_lists_the_deposit_cli_commands() {
    let help = run_ok(&["--help"]);
    for command in [
        "new-mnemonic",
        "existing-mnemonic",
        "generate-bls-to-execution-change",
        "generate-bls-to-execution-change-keystore",
        "exit-transaction-keystore",
        "exit-transaction-mnemonic",
        "partial-deposit",
    ] {
        assert!(help.contains(command), "{command} missing from:\n{help}");
    }
}

#[test]
fn existing_mnemonic_writes_keystores_and_deposit_data() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_str().unwrap();
    let stdout = run_ok(&[
        "existing-mnemonic",
        "--mnemonic",
        ABANDON,
        "--validator_start_index",
        "1",
        "--num_validators",
        "2",
        "--chain",
        "mainnet",
        "--keystore_password",
        "MyPasswordIs",
        "--folder",
        out,
    ]);
    assert!(stdout.contains("Your keys can be found at"), "{stdout}");

    let keys_dir = dir.path().join("validator_keys");
    let keystores = files_with_prefix(&keys_dir, "keystore-m_12381_3600_");
    assert_eq!(keystores.len(), 2);
    assert!(keystores[0]
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("keystore-m_12381_3600_1_0_0-"));
    let deposits = files_with_prefix(&keys_dir, "deposit_data-");
    assert_eq!(deposits.len(), 1);

    // Deposit data is deterministic: indices 1 and 2 of the 3-key golden case.
    let expected = read_json(&golden("keys_mainnet_bls_3").join("deposit_data.json"));
    let actual = read_json(&deposits[0]);
    assert_eq!(actual.as_array().unwrap().len(), 2);
    assert_eq!(actual[0], expected[1]);
    assert_eq!(actual[1], expected[2]);
}

#[test]
fn new_mnemonic_prints_the_mnemonic_and_writes_keys() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_str().unwrap();
    let stdout = run_ok(&[
        "new-mnemonic",
        "--num_validators",
        "1",
        "--chain",
        "hoodi",
        "--keystore_password",
        "MyPasswordIs",
        "--withdrawal_address",
        "0x00000000219ab540356cBB839Cbe05303d7705Fa",
        "--compounding",
        "--amount",
        "40",
        "--pbkdf2",
        "--folder",
        out,
    ]);
    let mnemonic = stdout
        .lines()
        .find(|l| l.split(' ').count() == 24)
        .unwrap_or_else(|| panic!("no 24-word line in:\n{stdout}"));
    assert!(mnemonic
        .split(' ')
        .all(|w| w.chars().all(|c| c.is_ascii_lowercase())));

    let keys_dir = dir.path().join("validator_keys");
    let keystore = read_json(&files_with_prefix(&keys_dir, "keystore-")[0]);
    assert_eq!(keystore["crypto"]["kdf"]["function"], "pbkdf2");
    let deposit = read_json(&files_with_prefix(&keys_dir, "deposit_data-")[0]);
    assert_eq!(deposit[0]["amount"], 40_000_000_000u64);
    assert_eq!(deposit[0]["network_name"], "hoodi");
    assert!(deposit[0]["withdrawal_credentials"]
        .as_str()
        .unwrap()
        .starts_with("02"));
}

#[test]
fn bls_change_from_mnemonic_matches_golden() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_str().unwrap();
    run_ok(&[
        "generate-bls-to-execution-change",
        "--chain",
        "mainnet",
        "--mnemonic",
        SISTER,
        "--validator_start_index",
        "0",
        "--validator_indices",
        "1,2",
        "--bls_withdrawal_credentials_list",
        CREDS,
        "--withdrawal_address",
        "0x3434343434343434343434343434343434343434",
        "--bls_to_execution_changes_folder",
        out,
    ]);
    let files = files_with_prefix(
        &dir.path().join("bls_to_execution_changes"),
        "bls_to_execution_change-",
    );
    assert_eq!(files.len(), 1);
    assert_eq!(
        read_json(&files[0]),
        read_json(&golden("btec_mainnet_2").join("bls_to_execution_change.json"))
    );
}

#[test]
fn bls_change_from_keystore_matches_golden() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_str().unwrap();
    run_ok(&[
        "generate-bls-to-execution-change-keystore",
        "--chain",
        "mainnet",
        "--keystore",
        mainnet_keystore().to_str().unwrap(),
        "--keystore_password",
        "MyPasswordIs",
        "--validator_index",
        "1",
        "--withdrawal_address",
        "0x3434343434343434343434343434343434343434",
        "--output_folder",
        out,
    ]);
    let files = files_with_prefix(
        &dir.path().join("bls_to_execution_changes_keystore"),
        "bls_to_execution_change_keystore_signature-1-",
    );
    assert_eq!(files.len(), 1);
    assert_eq!(
        read_json(&files[0]),
        read_json(
            &golden("btec_keystore_mainnet")
                .join("bls_to_execution_change_keystore_signature.json")
        )
    );
}

#[test]
fn exit_transaction_from_mnemonic_matches_golden() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_str().unwrap();
    let stdout = run_ok(&[
        "exit-transaction-mnemonic",
        "--chain",
        "mainnet",
        "--mnemonic",
        SISTER,
        "--validator_start_index",
        "0",
        "--validator_indices",
        "1 2",
        "--epoch",
        "1234",
        "--output_folder",
        out,
    ]);
    assert!(
        stdout.contains("Your exit transaction files can be found at"),
        "{stdout}"
    );
    let exit_dir = dir.path().join("exit_transactions");
    for index in [1, 2] {
        let files = files_with_prefix(&exit_dir, &format!("signed_exit_transaction-{index}-"));
        assert_eq!(files.len(), 1, "validator {index}");
        assert_eq!(
            read_json(&files[0]),
            read_json(
                &golden("exit_mnemonic_mainnet_2")
                    .join(format!("signed_exit_transaction-{index}.json"))
            )
        );
    }
}

#[test]
fn exit_transaction_from_keystore_matches_golden() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_str().unwrap();
    run_ok(&[
        "exit-transaction-keystore",
        "--chain",
        "mainnet",
        "--keystore",
        mainnet_keystore().to_str().unwrap(),
        "--keystore_password",
        "MyPasswordIs",
        "--validator_index",
        "1",
        "--epoch",
        "1234",
        "--output_folder",
        out,
    ]);
    let files = files_with_prefix(
        &dir.path().join("exit_transactions"),
        "signed_exit_transaction-1-",
    );
    assert_eq!(files.len(), 1);
    assert_eq!(
        read_json(&files[0]),
        read_json(&golden("exit_keystore_mainnet").join("signed_exit_transaction.json"))
    );
}

#[test]
fn partial_deposit_matches_golden() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_str().unwrap();
    run_ok(&[
        "partial-deposit",
        "--chain",
        "mainnet",
        "--keystore",
        mainnet_keystore().to_str().unwrap(),
        "--keystore_password",
        "MyPasswordIs",
        "--amount",
        "100.5",
        "--withdrawal_address",
        "0x00000000219ab540356cBB839Cbe05303d7705Fa",
        "--compounding",
        "--output_folder",
        out,
    ]);
    let files = files_with_prefix(&dir.path().join("partial_deposits"), "deposit_data-");
    assert_eq!(files.len(), 1);
    assert_eq!(
        read_json(&files[0]),
        read_json(&golden("partial_mainnet_compounding_100_5").join("deposit_data.json"))
    );
}

#[test]
fn partial_deposit_applies_the_gnosis_multiplier() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_str().unwrap();
    run_ok(&[
        "partial-deposit",
        "--chain",
        "gnosis",
        "--keystore",
        mainnet_keystore().to_str().unwrap(),
        "--keystore_password",
        "MyPasswordIs",
        "--amount",
        "2",
        "--withdrawal_address",
        "0x3434343434343434343434343434343434343434",
        "--compounding",
        "--output_folder",
        out,
    ]);
    let files = files_with_prefix(&dir.path().join("partial_deposits"), "deposit_data-");
    // 2 GNO is a 64e9 gwei deposit message, the same file as the 64e9 golden case.
    assert_eq!(
        read_json(&files[0]),
        read_json(&golden("partial_gnosis_compounding_2").join("deposit_data.json"))
    );
}

#[test]
fn non_interactive_mode_reports_missing_options_and_bad_input() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_str().unwrap();

    let stderr = run_err(&[
        "existing-mnemonic",
        "--num_validators",
        "1",
        "--keystore_password",
        "MyPasswordIs",
        "--folder",
        out,
    ]);
    assert!(stderr.contains("--mnemonic"), "{stderr}");

    let stderr = run_err(&[
        "exit-transaction-keystore",
        "--keystore",
        mainnet_keystore().to_str().unwrap(),
        "--keystore_password",
        "not the password",
        "--validator_index",
        "1",
        "--output_folder",
        out,
    ]);
    assert!(
        stderr.contains("The keystore password is incorrect."),
        "{stderr}"
    );

    let stderr = run_err(&[
        "existing-mnemonic",
        "--mnemonic",
        "abandon abandon",
        "--num_validators",
        "1",
        "--keystore_password",
        "MyPasswordIs",
        "--folder",
        out,
    ]);
    assert!(stderr.contains("not a valid mnemonic"), "{stderr}");

    let stderr = run_err(&[
        "existing-mnemonic",
        "--mnemonic",
        ABANDON,
        "--num_validators",
        "1",
        "--keystore_password",
        "short",
        "--folder",
        out,
    ]);
    assert!(stderr.contains("at least 12 characters"), "{stderr}");

    let stderr = run_err(&[
        "existing-mnemonic",
        "--mnemonic",
        ABANDON,
        "--num_validators",
        "1",
        "--keystore_password",
        "MyPasswordIs",
        "--folder",
        &format!("{out}/missing"),
    ]);
    assert!(stderr.contains("does not exist"), "{stderr}");

    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        0,
        "nothing written on error"
    );
}
