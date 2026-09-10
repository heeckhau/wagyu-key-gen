//! Command-line interface of Wagyu Key Gen.
//!
//! Mirrors the commands, options and output layout of `ethstaker-deposit-cli` 1.2.2 on top of
//! `wagyu-core`: the same subcommand names, the same `--snake_case` options, the same output
//! subfolders (`validator_keys`, `exit_transactions`, ...) and the same file contents.
//!
//! Differences from the Python CLI: prompts are English only, `--devnet_chain_setting` is not
//! supported, and `partial-deposit` applies the Gnosis multiplier to `--amount` like
//! `new-mnemonic` / `existing-mnemonic` do (the Python command does not).

use std::io::{self, BufRead, IsTerminal, Write};
use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use wagyu_core::btec::{GenerateBtecKeystoreRequest, GenerateBtecRequest};
use wagyu_core::chain::GWEI_PER_ETH;
use wagyu_core::deposit::GenerateKeysRequest;
use wagyu_core::exit::{GenerateExitFromKeystoreRequest, GenerateExitFromMnemonicRequest};
use wagyu_core::keystore::KdfChoice;
use wagyu_core::mnemonic::{language_from_name, Language};
use wagyu_core::partial_deposit::PartialDepositRequest;
use wagyu_core::validation::{
    deposit_amount_to_gwei, is_address, parse_bls_withdrawal_credentials_list,
    parse_validator_indices, validate_password,
};
use wagyu_core::Network;
use zeroize::Zeroizing;

const VALIDATOR_KEYS_FOLDER: &str = "validator_keys";
const BLS_CHANGES_FOLDER: &str = "bls_to_execution_changes";
const BLS_CHANGES_KEYSTORE_FOLDER: &str = "bls_to_execution_changes_keystore";
const EXIT_TRANSACTIONS_FOLDER: &str = "exit_transactions";
const PARTIAL_DEPOSITS_FOLDER: &str = "partial_deposits";

const CHAIN_PROMPT: &str = "Please choose the (mainnet or testnet) network/chain name";
const MNEMONIC_PROMPT: &str = "Please enter your mnemonic separated by spaces (\" \"). Note: you only need to enter the first 4 letters of each word if you'd prefer.";
const MNEMONIC_PASSWORD_PROMPT: &str =
    "Enter your mnemonic password (if you used one). Make sure you won't forget it, it can not be recovered.";
const MNEMONIC_PASSWORD_CONFIRM: &str = "Repeat your mnemonic password for confirmation. Providing a password here when you didn't use one initially, can result in lost keys (and therefore funds)!";
const KEYSTORE_PASSWORD_PROMPT: &str = "Create a password that secures your validator keystore(s). You will need to re-enter this to decrypt them when you setup your Ethereum validators.";
const KEYSTORE_PASSWORD_CONFIRM: &str = "Repeat your keystore password for confirmation";
const EXISTING_KEYSTORE_PASSWORD_PROMPT: &str =
    "Enter the password that is used to encrypt the provided keystore.";
const KEYSTORE_PATH_PROMPT: &str = "Please enter the location of your keystore file.";
const WITHDRAWAL_ADDRESS_CONFIRM: &str = "Repeat your withdrawal address for confirmation.";
const COMPOUNDING_PROMPT: &str = "Please enter yes if you want to generate compounding validators with 0x02 withdrawal credentials for a 2048 ETH maximum effective balance. Compounding validators and 0x02 withdrawal credentials are only supported on networks that have undergone the Pectra fork. Please type no or nothing if you want regular validators with 0x01 withdrawal credentials for a 32 ETH maximum effective balance.";
const VALIDATOR_INDICES_PROMPT: &str = "Please enter a list of the validator index number(s) of your validator(s) as identified on the beacon chain. Split multiple items with whitespaces or commas.";
const MISMATCH: &str = "Error: the two entered values do not match. Please type again.";
const CONNECTIVITY_WARNING: &str = "\n*** Internet connectivity detected ***\n\nTo mitigate the risk of unauthorized access and safeguard generated key material, it is strongly advised to run this tool in an offline, airgapped environment. Operating online increases susceptibility to potential theft or compromise.\n\nBy continuing, you are accepting responsibility for the risk.\n\nPress Enter to continue...";

#[derive(Parser)]
#[command(
    name = "wagyu-cli",
    version,
    about = "Generates Ethereum validator keys, deposit data, withdrawal credential changes and exit transactions, producing the same files as ethstaker-deposit-cli.",
    subcommand_required = true,
    arg_required_else_help = true
)]
struct Cli {
    /// The language you wish to use the CLI in. Only "english" is available.
    #[arg(long, global = true, default_value = "english")]
    language: String,

    /// Disables interactive prompts. Warning: With this flag, there will be no confirmation
    /// step(s) to verify the input value(s), and a new mnemonic is not asked back. This will
    /// also ignore the connectivity check. Please use it carefully.
    #[arg(long = "non_interactive", alias = "non-interactive", global = true)]
    non_interactive: bool,

    /// Disables internet connectivity check. Warning: It is strongly recommended not to use this
    /// tool with internet access. Ignoring this check can further the risk of theft and
    /// compromise of your generated key material.
    #[arg(
        long = "ignore_connectivity",
        alias = "ignore-connectivity",
        global = true
    )]
    ignore_connectivity: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a new mnemonic and keys
    NewMnemonic(NewMnemonicArgs),
    /// Generate (or recover) keys from an existing mnemonic
    ExistingMnemonic(ExistingMnemonicArgs),
    /// Generating the SignedBLSToExecutionChange data to enable withdrawals on Ethereum Beacon
    /// Chain.
    GenerateBlsToExecutionChange(BlsChangeArgs),
    /// Generating the SignedBLSToExecutionChangeKeystore data, signed with a keystore's signing
    /// key instead of the mnemonic's withdrawal key.
    GenerateBlsToExecutionChangeKeystore(BlsChangeKeystoreArgs),
    /// Generate an exit transaction, signed with a keystore, that can be used to exit a validator
    /// on Ethereum Beacon Chain.
    ExitTransactionKeystore(ExitKeystoreArgs),
    /// Generate exit transactions, signed with keys from a mnemonic, that can be used to exit
    /// validators on Ethereum Beacon Chain.
    ExitTransactionMnemonic(ExitMnemonicArgs),
    /// Generate a partial deposit with any amount at least 1 ether which will be signed by the
    /// provided validator keystore. This will append to the balance of the provided validator or
    /// initiate the creation of one.
    PartialDeposit(PartialDepositArgs),
}

#[derive(Args)]
struct MnemonicArgs {
    /// The mnemonic that you used to generate your keys. (It is recommended not to use this
    /// argument, and wait for the CLI to ask you for your mnemonic as otherwise it will appear
    /// in your shell history.)
    #[arg(long)]
    mnemonic: Option<String>,

    /// This is almost certainly not the argument you are looking for: it is for mnemonic
    /// passwords, not keystore passwords. Providing a password here when you didn't use one
    /// initially, can result in lost keys (and therefore funds)!
    #[arg(long = "mnemonic_password", alias = "mnemonic-password", hide = true)]
    mnemonic_password: Option<String>,

    /// The language of your mnemonic. If this is not provided we will attempt to determine it
    /// based on the mnemonic provided.
    #[arg(long = "mnemonic_language", alias = "mnemonic-language")]
    mnemonic_language: Option<String>,
}

#[derive(Args)]
struct ChainArg {
    /// The name of Ethereum PoS chain you are targeting. Use "mainnet" if you are depositing ETH
    #[arg(long)]
    chain: Option<String>,
}

#[derive(Args)]
struct KeystoreArgs {
    /// The keystore file associated with the validator you wish to sign with.
    #[arg(long)]
    keystore: Option<PathBuf>,

    /// The password that is used to encrypt the provided keystore. Note: It's not your mnemonic
    /// password. (It is recommended not to use this argument, and wait for the CLI to ask you
    /// for your password as otherwise it will appear in your shell history.)
    #[arg(long = "keystore_password", alias = "keystore-password")]
    keystore_password: Option<String>,
}

#[derive(Args)]
struct GenerateKeysArgs {
    /// The number of new validator keys you want to generate (you can always generate more
    /// later)
    #[arg(long = "num_validators", alias = "num-validators")]
    num_validators: Option<u32>,

    /// The folder path for the keystore(s) and deposit(s). Pointing to `./validator_keys` by
    /// default.
    #[arg(long, default_value = ".")]
    folder: PathBuf,

    #[command(flatten)]
    chain: ChainArg,

    /// The password that will secure your keystores. You will need to re-enter this to decrypt
    /// them when you setup your Ethereum validators. (It is recommended not to use this
    /// argument, and wait for the CLI to ask you for your password, as otherwise it will appear
    /// in your shell history.)
    #[arg(long = "keystore_password", alias = "keystore-password")]
    keystore_password: Option<String>,

    /// The Ethereum address that will be used in withdrawal. It typically starts with '0x'
    /// followed by 40 hexadecimal characters. Please make sure you have full control over the
    /// address you choose here. Once you set a withdrawal address on chain, it cannot be
    /// changed.
    #[arg(
        long = "withdrawal_address",
        visible_alias = "execution_address",
        alias = "eth1_withdrawal_address",
        alias = "withdrawal-address"
    )]
    withdrawal_address: Option<String>,

    /// Generates compounding validators with 0x02 withdrawal credentials for a 2048 ETH maximum
    /// effective balance. Use of this option requires a withdrawal address.
    #[arg(long, overrides_with = "regular_withdrawal")]
    compounding: bool,

    /// Generates regular validators with 0x01 withdrawal credentials for a 32 ETH maximum
    /// effective balance (the default).
    #[arg(
        long = "regular-withdrawal",
        alias = "regular_withdrawal",
        overrides_with = "compounding"
    )]
    regular_withdrawal: bool,

    /// The amount to deposit to these validators in ether denomination. Must be at least 1
    /// ether and can not have greater precision than 1 gwei. Use of this option requires
    /// compounding validators.
    #[arg(long)]
    amount: Option<String>,

    /// Uses the pbkdf2 hashing function instead of scrypt for generated keystore files.
    #[arg(long)]
    pbkdf2: bool,
}

#[derive(Args)]
struct NewMnemonicArgs {
    /// The language of the mnemonic word list
    #[arg(
        long = "mnemonic_language",
        alias = "mnemonic-language",
        default_value = "english"
    )]
    mnemonic_language: String,

    #[command(flatten)]
    keys: GenerateKeysArgs,
}

#[derive(Args)]
struct ExistingMnemonicArgs {
    #[command(flatten)]
    mnemonic: MnemonicArgs,

    /// Enter the index (key number) you wish to start generating more keys from. For example,
    /// if you've generated 4 keys in the past, you'd enter 4 here.
    #[arg(long = "validator_start_index", alias = "validator-start-index")]
    validator_start_index: Option<u32>,

    #[command(flatten)]
    keys: GenerateKeysArgs,
}

#[derive(Args)]
struct BlsChangeArgs {
    /// The folder path for the files. Pointing to `./bls_to_execution_changes` by default.
    #[arg(
        long = "bls_to_execution_changes_folder",
        alias = "bls-to-execution-changes-folder",
        default_value = "."
    )]
    bls_to_execution_changes_folder: PathBuf,

    #[command(flatten)]
    chain: ChainArg,

    #[command(flatten)]
    mnemonic: MnemonicArgs,

    /// The index position for the keys to start generating withdrawal credentials in ERC-2334
    /// format
    #[arg(long = "validator_start_index", alias = "validator-start-index")]
    validator_start_index: Option<u32>,

    /// A list of the validator index number(s) of the certain validator(s)
    #[arg(long = "validator_indices", alias = "validator-indices")]
    validator_indices: Option<String>,

    /// A list of 32-byte old BLS withdrawal credentials of the certain validator(s)
    #[arg(
        long = "bls_withdrawal_credentials_list",
        alias = "bls-withdrawal-credentials-list"
    )]
    bls_withdrawal_credentials_list: Option<String>,

    /// The Ethereum address that will be used in withdrawal. Once you set a withdrawal address
    /// on chain, it cannot be changed.
    #[arg(long = "withdrawal_address", alias = "withdrawal-address")]
    withdrawal_address: Option<String>,
}

#[derive(Args)]
struct BlsChangeKeystoreArgs {
    #[command(flatten)]
    chain: ChainArg,

    #[command(flatten)]
    keystore: KeystoreArgs,

    /// The validator index number of your validator as identified on the beacon chain.
    #[arg(long = "validator_index", alias = "validator-index")]
    validator_index: Option<u64>,

    /// The Ethereum address that will be used in withdrawal. Once you set a withdrawal address
    /// on chain, it cannot be changed.
    #[arg(long = "withdrawal_address", alias = "withdrawal-address")]
    withdrawal_address: Option<String>,

    /// Folder where you want to save the bls keystore change. Pointing to
    /// `./bls_to_execution_changes_keystore` by default.
    #[arg(long = "output_folder", alias = "output-folder", default_value = ".")]
    output_folder: PathBuf,
}

#[derive(Args)]
struct ExitKeystoreArgs {
    #[command(flatten)]
    chain: ChainArg,

    #[command(flatten)]
    keystore: KeystoreArgs,

    /// The validator index corresponding to the provided keystore.
    #[arg(long = "validator_index", alias = "validator-index")]
    validator_index: Option<u64>,

    /// The epoch of when the exit transaction will be valid. The transaction will always be
    /// valid by default.
    #[arg(long, default_value_t = 0)]
    epoch: u64,

    /// The folder path where the exit transactions will be saved to. Pointing to
    /// `./exit_transactions` by default.
    #[arg(long = "output_folder", alias = "output-folder", default_value = ".")]
    output_folder: PathBuf,
}

#[derive(Args)]
struct ExitMnemonicArgs {
    #[command(flatten)]
    chain: ChainArg,

    #[command(flatten)]
    mnemonic: MnemonicArgs,

    /// Enter the index (key number) which you used when you created your keys. The default
    /// value is 0.
    #[arg(long = "validator_start_index", alias = "validator-start-index")]
    validator_start_index: Option<u32>,

    /// A list of the validator index number(s) of the certain validator(s)
    #[arg(long = "validator_indices", alias = "validator-indices")]
    validator_indices: Option<String>,

    /// The epoch of when the exit transaction will be valid. The transaction will always be
    /// valid by default.
    #[arg(long, default_value_t = 0)]
    epoch: u64,

    /// The folder path where the exit transactions will be saved to. Pointing to
    /// `./exit_transactions` by default.
    #[arg(long = "output_folder", alias = "output-folder", default_value = ".")]
    output_folder: PathBuf,
}

#[derive(Args)]
struct PartialDepositArgs {
    #[command(flatten)]
    chain: ChainArg,

    #[command(flatten)]
    keystore: KeystoreArgs,

    /// The amount to deposit to this validator in ether denomination. Must be at least 1 ether
    /// and can not have greater precision than 1 gwei. Default is 32 ether (1 GNO on Gnosis
    /// chains).
    #[arg(long)]
    amount: Option<String>,

    /// The withdrawal address of the validator. If you wish to create a validator with 0x00
    /// credentials use the new-mnemonic or existing-mnemonic command.
    #[arg(
        long = "withdrawal_address",
        visible_alias = "execution_address",
        alias = "eth1_withdrawal_credentials",
        alias = "withdrawal-address"
    )]
    withdrawal_address: Option<String>,

    /// Signs for compounding (0x02) withdrawal credentials instead of regular (0x01) ones.
    #[arg(long, overrides_with = "regular_withdrawal")]
    compounding: bool,

    /// Signs for regular (0x01) withdrawal credentials (the default).
    #[arg(
        long = "regular-withdrawal",
        alias = "regular_withdrawal",
        overrides_with = "compounding"
    )]
    regular_withdrawal: bool,

    /// The folder path where the partial deposit will be saved to. Pointing to
    /// `./partial_deposits` by default.
    #[arg(long = "output_folder", alias = "output-folder", default_value = ".")]
    output_folder: PathBuf,
}

/// Anything that stops a command; printed as `Error: <text>` with exit status 1.
struct Failure(String);

impl<E: std::fmt::Display> From<E> for Failure {
    fn from(e: E) -> Self {
        Failure(e.to_string())
    }
}

type CliResult<T> = Result<T, Failure>;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure(text)) => {
            eprintln!("\nError: {text}\n");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> CliResult<()> {
    if !cli.language.trim().eq_ignore_ascii_case("english") {
        return Err(Failure(format!(
            "Language '{}' is not available; only english is.",
            cli.language
        )));
    }
    let prompter = Prompter {
        interactive: !cli.non_interactive,
    };
    if !cli.ignore_connectivity && !cli.non_interactive {
        check_connectivity(&prompter);
    }
    match cli.command {
        Command::NewMnemonic(args) => new_mnemonic(&prompter, args),
        Command::ExistingMnemonic(args) => existing_mnemonic(&prompter, args),
        Command::GenerateBlsToExecutionChange(args) => bls_change(&prompter, args),
        Command::GenerateBlsToExecutionChangeKeystore(args) => bls_change_keystore(&prompter, args),
        Command::ExitTransactionKeystore(args) => exit_keystore(&prompter, args),
        Command::ExitTransactionMnemonic(args) => exit_mnemonic(&prompter, args),
        Command::PartialDeposit(args) => partial_deposit(&prompter, args),
    }
}

/// Warns, like the Python CLI, when the machine can resolve a public host name.
fn check_connectivity(prompter: &Prompter) {
    if ("icann.org", 80).to_socket_addrs().is_ok() {
        prompter.pause(CONNECTIVITY_WARNING);
    }
}

// ---------------------------------------------------------------------------------------------
// Commands

fn new_mnemonic(prompter: &Prompter, args: NewMnemonicArgs) -> CliResult<()> {
    let language = language_from_name(&args.mnemonic_language)?;
    let keys = collect_generate_keys(prompter, args.keys)?;

    let mnemonic = wagyu_core::create_mnemonic(language)?;
    prompter.clear();
    println!("This is your mnemonic (seed phrase). Write it down and store it safely. It is the ONLY way to retrieve your deposit.\n");
    println!("{}\n", mnemonic.as_str());
    if prompter.interactive {
        prompter.pause("Press Enter when you have written down your mnemonic.");
        loop {
            prompter.clear();
            let typed = prompter.text("Please type your mnemonic (separated by spaces) to confirm you have written it down. Note: you only need to enter the first 4 letters of each word if you'd prefer.")?;
            match wagyu_core::reconstruct_mnemonic(&typed, Some(language)) {
                Ok(m) if m.as_str() == mnemonic.as_str() => break,
                _ => {
                    prompter.clear();
                    println!(
                        "That did not match. This is your mnemonic:\n\n{}\n",
                        mnemonic.as_str()
                    );
                    prompter.pause("Press Enter when you have written down your mnemonic.");
                }
            }
        }
        prompter.clear();
    }

    generate_keys(
        mnemonic,
        Zeroizing::new(String::new()),
        Some(language),
        0,
        keys,
    )
}

fn existing_mnemonic(prompter: &Prompter, args: ExistingMnemonicArgs) -> CliResult<()> {
    let (mnemonic, mnemonic_password, language) = collect_mnemonic(prompter, args.mnemonic)?;
    let start_index = match args.validator_start_index {
        Some(i) => i,
        None if prompter.interactive => prompter.confirmed_u32(
            "Enter the index (key number) you wish to start generating more keys from. For example, if you've generated 4 keys in the past, you'd enter 4 here.",
            "Please repeat the validator start index to confirm",
        )?,
        None => 0,
    };
    let keys = collect_generate_keys(prompter, args.keys)?;
    generate_keys(mnemonic, mnemonic_password, language, start_index, keys)
}

fn bls_change(prompter: &Prompter, args: BlsChangeArgs) -> CliResult<()> {
    let folder = output_subfolder(&args.bls_to_execution_changes_folder, BLS_CHANGES_FOLDER)?;
    let network = collect_chain(prompter, args.chain)?;
    let (mnemonic, mnemonic_password, language) = collect_mnemonic(prompter, args.mnemonic)?;
    let start_index = match args.validator_start_index {
        Some(i) => i,
        None if prompter.interactive => prompter.confirmed_u32(
            "Please enter the index position for the keys to start generating withdrawal credentials in ERC-2334 format.",
            "Please repeat the index to confirm",
        )?,
        None => 0,
    };
    let validator_indices = parse_validator_indices(&prompter.required(
        args.validator_indices,
        VALIDATOR_INDICES_PROMPT,
        "--validator_indices",
    )?)?;
    let bls_withdrawal_credentials = parse_bls_withdrawal_credentials_list(&prompter.required(
        args.bls_withdrawal_credentials_list,
        "Please enter a list of the old BLS withdrawal credentials of your validator(s). Split multiple items with whitespaces or commas. The withdrawal credentials are in hexadecimal encoded form.",
        "--bls_withdrawal_credentials_list",
    )?)?;
    let withdrawal_address = collect_required_address(
        prompter,
        args.withdrawal_address,
        "Please enter the withdrawal address. Note that you CANNOT change it once you have set it on chain.",
    )?;

    println!("Creating your SignedBLSToExecutionChange.");
    let path = wagyu_core::generate_bls_to_execution_change(&GenerateBtecRequest {
        folder,
        network,
        mnemonic,
        mnemonic_password,
        mnemonic_language: language,
        start_index,
        validator_indices,
        bls_withdrawal_credentials,
        withdrawal_address,
    })?;
    println!(
        "\nSuccess!\nYour SignedBLSToExecutionChange JSON file can be found at: {}",
        path.display()
    );
    Ok(())
}

fn bls_change_keystore(prompter: &Prompter, args: BlsChangeKeystoreArgs) -> CliResult<()> {
    let folder = output_subfolder(&args.output_folder, BLS_CHANGES_KEYSTORE_FOLDER)?;
    let network = collect_chain(prompter, args.chain)?;
    let (keystore_path, keystore_password) = collect_keystore(prompter, args.keystore)?;
    let validator_index = collect_validator_index(prompter, args.validator_index)?;
    let withdrawal_address = collect_required_address(
        prompter,
        args.withdrawal_address,
        "Please enter the withdrawal address. Note that you CANNOT change it once you have set it on chain.",
    )?;

    println!("Creating your SignedBLSToExecutionChangeKeystore.");
    let path =
        wagyu_core::generate_bls_to_execution_change_keystore(&GenerateBtecKeystoreRequest {
            folder,
            network,
            keystore_path,
            keystore_password,
            validator_index,
            withdrawal_address,
        })?;
    println!(
        "\nSuccess!\nYour SignedBLSToExecutionChangeKeystore JSON file can be found at: {}",
        path.display()
    );
    Ok(())
}

fn exit_keystore(prompter: &Prompter, args: ExitKeystoreArgs) -> CliResult<()> {
    let folder = output_subfolder(&args.output_folder, EXIT_TRANSACTIONS_FOLDER)?;
    let network = collect_chain(prompter, args.chain)?;
    let (keystore_path, keystore_password) = collect_keystore(prompter, args.keystore)?;
    let validator_index = collect_validator_index(prompter, args.validator_index)?;

    println!("\nCreating your exit transaction...");
    let path =
        wagyu_core::generate_exit_transaction_from_keystore(&GenerateExitFromKeystoreRequest {
            folder,
            network,
            keystore_path,
            keystore_password,
            validator_index,
            epoch: args.epoch,
        })?;
    println!(
        "\nSuccess!\nYour exit transaction file can be found at: {}",
        path.display()
    );
    Ok(())
}

fn exit_mnemonic(prompter: &Prompter, args: ExitMnemonicArgs) -> CliResult<()> {
    let folder = output_subfolder(&args.output_folder, EXIT_TRANSACTIONS_FOLDER)?;
    let network = collect_chain(prompter, args.chain)?;
    let (mnemonic, mnemonic_password, language) = collect_mnemonic(prompter, args.mnemonic)?;
    let start_index = match args.validator_start_index {
        Some(i) => i,
        None => prompter
            .optional(
                "Enter the index (key number) which you used when you created your keys. The default value is 0.",
            )?
            .map(|s| s.trim().parse::<u32>())
            .transpose()
            .map_err(|_| Failure("That is not a positive integer.".to_string()))?
            .unwrap_or(0),
    };
    let validator_indices = parse_validator_indices(&prompter.required(
        args.validator_indices,
        VALIDATOR_INDICES_PROMPT,
        "--validator_indices",
    )?)?;

    println!("Creating your exit transactions...");
    let paths = wagyu_core::generate_exit_transactions(&GenerateExitFromMnemonicRequest {
        folder: folder.clone(),
        network,
        mnemonic,
        mnemonic_password,
        mnemonic_language: language,
        start_index,
        validator_indices,
        epoch: args.epoch,
    })?;
    println!(
        "\nSuccess!\nYour exit transaction files can be found at: {}",
        folder.display()
    );
    for path in paths {
        println!("  {}", path.display());
    }
    Ok(())
}

fn partial_deposit(prompter: &Prompter, args: PartialDepositArgs) -> CliResult<()> {
    let folder = output_subfolder(&args.output_folder, PARTIAL_DEPOSITS_FOLDER)?;
    let network = collect_chain(prompter, args.chain)?;
    let chain = network.setting();
    let (keystore_path, keystore_password) = collect_keystore(prompter, args.keystore)?;
    let default_amount = format_ether(chain.min_activation_amount_gwei);
    let amount = match args.amount {
        Some(a) => a,
        None => prompter
            .optional(&format!(
                "Please enter the amount you wish to deposit to this validator. Must be at least {} and can not have greater precision than 1 gwei. {default_amount} is required to activate a new validator [{default_amount}]",
                format_ether(chain.min_deposit_amount_gwei)
            ))?
            .unwrap_or(default_amount),
    };
    let amount_gwei = deposit_amount_to_gwei(amount.trim(), chain)?;
    let withdrawal_address = collect_required_address(
        prompter,
        args.withdrawal_address,
        "Please enter the withdrawal address. If you wish to create a validator with 0x00 credentials use the new-mnemonic or existing-mnemonic command.",
    )?;
    let compounding = if args.compounding || args.regular_withdrawal {
        args.compounding
    } else {
        prompter.yes_no(COMPOUNDING_PROMPT)?
    };

    println!("\nCreating your partial deposit...");
    let path = wagyu_core::generate_partial_deposit(&PartialDepositRequest {
        folder,
        network,
        keystore_path,
        keystore_password,
        amount_gwei,
        withdrawal_address,
        compounding,
    })?;
    println!(
        "\nSuccess!\nYour partial deposit file can be found at: {}",
        path.display()
    );
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Shared argument collection

/// Everything `new-mnemonic` and `existing-mnemonic` need besides the mnemonic itself.
struct GenerateKeysInput {
    num_validators: u32,
    folder: PathBuf,
    network: Network,
    keystore_password: Zeroizing<String>,
    withdrawal_address: Option<String>,
    compounding: bool,
    amount_gwei: u64,
    kdf: KdfChoice,
}

fn collect_generate_keys(
    prompter: &Prompter,
    args: GenerateKeysArgs,
) -> CliResult<GenerateKeysInput> {
    let num_validators = match args.num_validators {
        Some(n) => n,
        None => prompter
            .required(
                None,
                "Please choose how many new validators you wish to run",
                "--num_validators",
            )?
            .trim()
            .parse::<u32>()
            .map_err(|_| Failure("That is not a positive integer.".to_string()))?,
    };
    if num_validators == 0 {
        return Err(Failure(
            "The number of validators must be at least 1.".to_string(),
        ));
    }
    let folder = output_subfolder(&args.folder, VALIDATOR_KEYS_FOLDER)?;
    let network = collect_chain(prompter, args.chain)?;
    let chain = network.setting();

    let keystore_password: Zeroizing<String> = match args.keystore_password {
        Some(p) => Zeroizing::new(p),
        None => loop {
            let password = prompter.confirmed_secret(
                KEYSTORE_PASSWORD_PROMPT,
                KEYSTORE_PASSWORD_CONFIRM,
                "--keystore_password",
            )?;
            match validate_password(&password) {
                Ok(()) => break password,
                Err(e) => {
                    if !prompter.interactive {
                        return Err(e.into());
                    }
                    println!("{e} Please retype.");
                }
            }
        },
    };
    validate_password(&keystore_password)?;

    let withdrawal_address = match args.withdrawal_address {
        Some(a) => Some(a).filter(|a| !a.trim().is_empty()),
        None => collect_optional_address(
            prompter,
            "Please enter the optional withdrawal address. Note that you CANNOT change it once you have set it on chain.",
        )?,
    };
    let compounding = match (
        &withdrawal_address,
        args.compounding,
        args.regular_withdrawal,
    ) {
        (None, true, _) => return Err(wagyu_core::Error::MissingAddress.into()),
        (None, _, _) => false,
        (Some(_), true, _) => true,
        (Some(_), _, true) => false,
        (Some(_), false, false) => prompter.yes_no(COMPOUNDING_PROMPT)?,
    };
    // Without compounding credentials the deposit is always the activation amount.
    let amount_gwei = if compounding {
        let amount = match args.amount {
            Some(a) => a,
            None => {
                let default = format_ether(chain.min_activation_amount_gwei);
                prompter
                    .optional(&format!(
                        "Please enter the amount you wish to deposit to these validators. Must be at least {} and can not have greater precision than 1 gwei. {default} is required to activate a new validator [{default}]",
                        format_ether(chain.min_deposit_amount_gwei)
                    ))?
                    .unwrap_or(default)
            }
        };
        deposit_amount_to_gwei(amount.trim(), chain)?
    } else {
        chain.default_deposit_amount_gwei()
    };

    Ok(GenerateKeysInput {
        num_validators,
        folder,
        network,
        keystore_password,
        withdrawal_address,
        compounding,
        amount_gwei,
        kdf: if args.pbkdf2 {
            KdfChoice::Pbkdf2
        } else {
            KdfChoice::Scrypt
        },
    })
}

fn generate_keys(
    mnemonic: Zeroizing<String>,
    mnemonic_password: Zeroizing<String>,
    mnemonic_language: Option<Language>,
    start_index: u32,
    keys: GenerateKeysInput,
) -> CliResult<()> {
    println!("Creating your keys.");
    let output = wagyu_core::generate_keys(&GenerateKeysRequest {
        mnemonic,
        mnemonic_password,
        mnemonic_language,
        start_index,
        count: keys.num_validators,
        amount_gwei: keys.amount_gwei,
        network: keys.network,
        keystore_password: keys.keystore_password,
        withdrawal_address: keys.withdrawal_address,
        compounding: keys.compounding,
        folder: keys.folder.clone(),
        kdf: keys.kdf,
    })?;
    println!(
        "\nSuccess!\nYour keys can be found at: {}",
        keys.folder.display()
    );
    println!("  {}", output.deposit_data_file.display());
    for path in output.keystore_files {
        println!("  {}", path.display());
    }
    Ok(())
}

fn collect_mnemonic(
    prompter: &Prompter,
    args: MnemonicArgs,
) -> CliResult<(Zeroizing<String>, Zeroizing<String>, Option<Language>)> {
    let language = args
        .mnemonic_language
        .as_deref()
        .map(language_from_name)
        .transpose()?;
    let typed = Zeroizing::new(prompter.required(args.mnemonic, MNEMONIC_PROMPT, "--mnemonic")?);
    // Validate early so a typo is reported before any other prompt.
    let mnemonic = wagyu_core::reconstruct_mnemonic(&typed, language)?;
    let password = match args.mnemonic_password {
        Some(p) => Zeroizing::new(p),
        None if prompter.interactive => prompter.confirmed_secret(
            MNEMONIC_PASSWORD_PROMPT,
            MNEMONIC_PASSWORD_CONFIRM,
            "--mnemonic_password",
        )?,
        None => Zeroizing::new(String::new()),
    };
    Ok((mnemonic, password, language))
}

fn collect_chain(prompter: &Prompter, arg: ChainArg) -> CliResult<Network> {
    let name = match arg.chain {
        Some(c) => c,
        None => prompter
            .optional(&format!("{CHAIN_PROMPT} [mainnet]"))?
            .unwrap_or_else(|| "mainnet".to_string()),
    };
    Ok(Network::from_name(&name)?)
}

fn collect_keystore(
    prompter: &Prompter,
    args: KeystoreArgs,
) -> CliResult<(PathBuf, Zeroizing<String>)> {
    let path = match args.keystore {
        Some(p) => p,
        None => PathBuf::from(
            prompter
                .required(None, KEYSTORE_PATH_PROMPT, "--keystore")?
                .trim(),
        ),
    };
    if !path.is_file() {
        return Err(Failure(
            "No file was found. Please verify the provided path and try again.".to_string(),
        ));
    }
    let password = match args.keystore_password {
        Some(p) => Zeroizing::new(p),
        None => prompter.secret(EXISTING_KEYSTORE_PASSWORD_PROMPT, "--keystore_password")?,
    };
    Ok((path, password))
}

fn collect_validator_index(prompter: &Prompter, arg: Option<u64>) -> CliResult<u64> {
    match arg {
        Some(i) => Ok(i),
        None => prompter
            .required(
                None,
                "Please enter the validator index of your validator as identified on the beacon chain.",
                "--validator_index",
            )?
            .trim()
            .parse::<u64>()
            .map_err(|_| Failure("That is not a positive integer.".to_string())),
    }
}

fn collect_required_address(
    prompter: &Prompter,
    arg: Option<String>,
    prompt: &str,
) -> CliResult<String> {
    let address = match arg {
        Some(a) => a,
        None => loop {
            let typed = prompter.required(None, prompt, "--withdrawal_address")?;
            let typed = typed.trim().to_string();
            if !is_address(&typed) {
                if !prompter.interactive {
                    return Err(wagyu_core::Error::InvalidAddress.into());
                }
                println!("{}", wagyu_core::Error::InvalidAddress);
                continue;
            }
            if !prompter.interactive || prompter.text(WITHDRAWAL_ADDRESS_CONFIRM)?.trim() == typed {
                break typed;
            }
            println!("{MISMATCH}");
        },
    };
    if !is_address(&address) {
        return Err(wagyu_core::Error::InvalidAddress.into());
    }
    println!("**[Warning] you are setting a withdrawal address. Please ensure that you have full control over this address.**");
    Ok(address)
}

fn collect_optional_address(prompter: &Prompter, prompt: &str) -> CliResult<Option<String>> {
    if !prompter.interactive {
        return Ok(None);
    }
    loop {
        let typed = prompter.text(prompt)?.trim().to_string();
        if typed.is_empty() {
            return Ok(None);
        }
        if !is_address(&typed) {
            println!("{}", wagyu_core::Error::InvalidAddress);
            continue;
        }
        if prompter.text(WITHDRAWAL_ADDRESS_CONFIRM)?.trim() == typed {
            println!("**[Warning] you are setting a withdrawal address. Please ensure that you have full control over this address.**");
            return Ok(Some(typed));
        }
        println!("{MISMATCH}");
    }
}

/// `<base>/<name>`, created if missing. `base` itself must already exist, like the Python
/// `click.Path(exists=True)` options.
fn output_subfolder(base: &Path, name: &str) -> CliResult<PathBuf> {
    if !base.is_dir() {
        return Err(Failure(format!(
            "Directory '{}' does not exist.",
            base.display()
        )));
    }
    Ok(base.join(name))
}

fn format_ether(gwei: u64) -> String {
    let whole = gwei / GWEI_PER_ETH;
    let frac = gwei % GWEI_PER_ETH;
    if frac == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{frac:09}")
            .trim_end_matches('0')
            .to_string()
    }
}

// ---------------------------------------------------------------------------------------------
// Terminal interaction

struct Prompter {
    interactive: bool,
}

impl Prompter {
    /// A value that must come from the option in non-interactive mode and is prompted otherwise.
    fn required(&self, value: Option<String>, prompt: &str, option: &str) -> CliResult<String> {
        match value {
            Some(v) => Ok(v),
            None if self.interactive => self.text(prompt),
            None => Err(Failure(format!(
                "Missing option '{option}' (prompts are disabled by --non_interactive)."
            ))),
        }
    }

    /// A prompt that may be left empty; `None` in non-interactive mode or on an empty answer.
    fn optional(&self, prompt: &str) -> CliResult<Option<String>> {
        if !self.interactive {
            return Ok(None);
        }
        let answer = self.text(prompt)?;
        Ok(Some(answer).filter(|a| !a.trim().is_empty()))
    }

    fn text(&self, prompt: &str) -> CliResult<String> {
        print!("{prompt}: ");
        io::stdout().flush().ok();
        let mut line = String::new();
        let read = io::stdin()
            .lock()
            .read_line(&mut line)
            .map_err(|e| Failure(format!("cannot read input: {e}")))?;
        if read == 0 {
            return Err(Failure("No input available.".to_string()));
        }
        Ok(line.trim_end_matches(['\r', '\n']).to_string())
    }

    fn secret(&self, prompt: &str, option: &str) -> CliResult<Zeroizing<String>> {
        if !self.interactive {
            return Err(Failure(format!(
                "Missing option '{option}' (prompts are disabled by --non_interactive)."
            )));
        }
        if io::stdin().is_terminal() {
            rpassword::prompt_password(format!("{prompt}: "))
                .map(Zeroizing::new)
                .map_err(|e| Failure(format!("cannot read input: {e}")))
        } else {
            self.text(prompt).map(Zeroizing::new)
        }
    }

    fn confirmed_secret(
        &self,
        prompt: &str,
        confirm: &str,
        option: &str,
    ) -> CliResult<Zeroizing<String>> {
        loop {
            let first = self.secret(prompt, option)?;
            let second = self.secret(confirm, option)?;
            if first.as_str() == second.as_str() {
                return Ok(first);
            }
            println!("{MISMATCH}");
        }
    }

    /// An integer typed twice; only used interactively, non-interactive callers use a default.
    fn confirmed_u32(&self, prompt: &str, confirm: &str) -> CliResult<u32> {
        loop {
            let first = self.text(prompt)?;
            let value: u32 = first
                .trim()
                .parse()
                .map_err(|_| Failure("That is not a positive integer.".to_string()))?;
            if self.text(confirm)?.trim() == first.trim() {
                return Ok(value);
            }
            println!("{MISMATCH}");
        }
    }

    fn yes_no(&self, prompt: &str) -> CliResult<bool> {
        if !self.interactive {
            return Ok(false);
        }
        loop {
            let answer = self.text(&format!("{prompt} [no]"))?;
            match answer.trim().to_ascii_lowercase().as_str() {
                "" | "n" | "no" | "false" | "0" => return Ok(false),
                "y" | "yes" | "true" | "1" => return Ok(true),
                _ => println!("This is an invalid value for a yes/no question."),
            }
        }
    }

    fn pause(&self, message: &str) {
        if self.interactive {
            println!("{message}");
            let mut line = String::new();
            let _ = io::stdin().lock().read_line(&mut line);
        }
    }

    /// Clears the terminal so a displayed mnemonic does not stay on screen.
    fn clear(&self) {
        if self.interactive && io::stdout().is_terminal() {
            print!("\x1B[2J\x1B[H");
            io::stdout().flush().ok();
        }
    }
}
