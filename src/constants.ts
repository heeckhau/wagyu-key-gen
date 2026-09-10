import { StepKey } from './types';
import { Network, NetworkConfig } from './types';

export const MNEMONIC_ERROR_SEARCH = "That is not a valid mnemonic";
export const VALID_MNEMONIC_LENGTHS = [12, 15, 18, 21, 24];
export const ETH_TO_GWEI = 10 ** 9;

export const getDepositAmountLimits = (network: Network) => {
  const multiplier = NetworkConfig[network].multiplier;
  return {
    min: 1 / multiplier,
    max: 2048 / multiplier,
  };
};

export const formatDepositAmountError = (network: Network): string => {
  const { min, max } = getDepositAmountLimits(network);
  return `Amount must be between ${min} and ${max}.`;
};

export const formatAmountTooltip = (network: Network): string => {
  const { min, max } = getDepositAmountLimits(network);
  return `Enter the amount you would like to initially deposit for each validator. This value must be between ${min} and ${max} and can not have greater precision than 1 gwei. You must have withdrawal credentials defined and validator type set to "compounding". You can "top up" a validator with additional funds after the initial deposit is processed.`;
};

export const formatCompoundingTooltip = (network: Network): string => {
  const { max } = getDepositAmountLimits(network);
  const value = 32 / NetworkConfig[network].multiplier;
  return `Compounding Credentials increases your maximum effective balance from ${value} to ${max} ETH.`;
};

export const formatTopUpAmountTooltip = (network: Network): string => {
  const { min, max } = getDepositAmountLimits(network);
  const cap = 32 / NetworkConfig[network].multiplier;
  return `The amount to deposit on top of the validator's current balance. This value must be between ${min} and ${max} and can not have greater precision than 1 gwei. Only validators with compounding (0x02) credentials can hold an effective balance above ${cap}; a regular (0x01) validator sweeps anything above that back to its withdrawal address.`;
};

export const errors = {
	MNEMONIC_LENGTH_ERROR: `The Secret Recovery Phrase must be ${VALID_MNEMONIC_LENGTHS.slice(0, -1).join(", ")}, or ${VALID_MNEMONIC_LENGTHS.slice(-1)} words in length. Please verify each word and try again.`,
	INVALID_MNEMONIC_ERROR: "The Secret Recovery Phrase provided is invalid. Please double check each word for any spelling errors.",
	MNEMONICS_DONT_MATCH: "The Secret Recovery Phrase you entered does not match what was given to you. Please try again.",
	NUMBER_OF_KEYS: "Please input a number between 1 and 1000.",
	ADDRESS_FORMAT_ERROR: "Please enter a valid Ethereum address.",
	WITHDRAW_ADDRESS_REQUIRED: "Please enter an Ethereum address.",
	PASSWORD_STRENGTH: "Password must be at least 12 characters.",
	PASSWORD_MATCH: "Passwords don't match.",
	STARTING_INDEX: "Please input start index.",
	NON_NEGATIVE_INDEX: "Start index can not be a negative number",
	INDICES: "Please input indices.",
	INDICES_FORMAT: "Please input indices with digits only.",
	INDICES_LENGTH: "The amount of indices must match the amount of BLS credentials",
	BLS_CREDENTIALS: "Please input BLS credentials.",
	BLS_CREDENTIALS_FORMAT: "Please enter valid BLS credentials.",
	BLS_CREDENTIALS_NO_MATCH: "Those BLS credentials do not match those we can derive from your Secret Recovery Phrase.",
	FOLDER: "Please select a folder.",
	FOLDER_DOES_NOT_EXISTS: "Folder does not exist. Select an existing folder.",
	FOLDER_IS_NOT_WRITABLE: "Cannot write in this folder. Select a folder in which you have write permission.",
	KEYSTORE_REQUIRED: "Please select a keystore file.",
	KEYSTORE_PASSWORD_REQUIRED: "Please enter the keystore password.",
	VALIDATOR_INDEX: "Please input a non-negative validator index.",
	EPOCH: "Please input a non-negative epoch.",
};

export const tooltips = {
	IMPORT_MNEMONIC: "If you've already created a Secret Recovery Phrase, you can use it to regenerate your original keys, create more keys, or generate a BLS to execution change by importing the phrase here.",
	NUMBER_OF_KEYS: "Enter how many new validator keys you'd like to create.",
	PASSWORD: "Pick a strong password (at least 12 characters) that will be used to protect your keys.",
	STARTING_INDEX: "Each key is created sequentially, so we need to know how many you've created with this Secret Recovery Phrase in the past in order to create some new ones for you.",
	ETH1_WITHDRAW_ADDRESS: "An optional Ethereum address for the withdrawal credentials.",
	BTEC_WITHDRAW_ADDRESS: "An Ethereum address for withdrawal. There is where your validator balance and rewards will go.",
	OFFLINE: "You want to avoid exposing your Secret Recovery Phrase to any system that can send it online and compromise your security. Booting from a live OS that does not connect to any network on a USB drive is an easy way to achieve that. You can copy the resulting files on USB drives. You might want to avoid storing your Secret Recovery Phrase electronically.",
	BTEC_START_INDEX: "The index position for the keys to start generating withdrawal credentials. If you only created 1 validator key using this Secret Recovery Phrase, this is likely going to be 0. If you created many validator keys, this could be a higher value from where you want to start in the list of validators derived from your Secret Recovery Phrase.",
	BTEC_INDICES: "A list of the chosen validator index number(s) as identified on the beacon chain. You can find your validator indice on beaconcha.in website on your validator page. It will be at the top of that page the form of a title like Validator XXXXX, where XXXXX is going to be your indice.",
	BLS_CREDENTIALS: "A list of the old BLS withdrawal credentials of the given validator(s). You can find your validator BLS withdrawal credentials on beaconcha.in website on your validator page. It will be in the Deposits tab and it should start with 0x00.",
	USE_KEYSTORE: "If you have a validator's keystore file (keystore-m_12381_3600_x_0_0-xxxx.json) and its password, you can exit that validator, top up its balance, or add a withdrawal address without typing your Secret Recovery Phrase.",
	KEYSTORE: "The keystore-m_12381_3600_x_0_0-xxxx.json file that was created together with the validator's deposit data. It contains the validator's signing key, protected by the password you chose at the time.",
	KEYSTORE_PASSWORD: "The password you chose when the keystore file was created.",
	EXIT_START_INDEX: "The index position for the keys to start signing exits from. If you only created 1 validator key using this Secret Recovery Phrase, this is likely going to be 0. If you created many validator keys, this could be a higher value from where you want to start in the list of validators derived from your Secret Recovery Phrase.",
	EXIT_INDICES: "A list of the chosen validator index number(s) as identified on the beacon chain, one per key starting at the start index. You can find your validator index on beaconcha.in website on your validator page, in the title Validator XXXXX.",
	EXIT_VALIDATOR_INDEX: "The validator index number as identified on the beacon chain. You can find it on beaconcha.in website on your validator page, in the title Validator XXXXX.",
	EXIT_EPOCH: "The earliest epoch at which the exit can be processed. Leave it at 0 to exit as soon as the network accepts the message.",
	PARTIAL_DEPOSIT_WITHDRAW_ADDRESS: "The withdrawal address of the validator being topped up. It must match the validator's current withdrawal credentials, otherwise the deposit will not be credited to it.",
	BTEC_KEYSTORE_VALIDATOR_INDEX: "The validator index number as identified on the beacon chain for the validator in this keystore.",
};

export const stepLabels = {
	[StepKey.MnemonicImport]: 'Import Secret Recovery Phrase',
	[StepKey.MnemonicGeneration]: 'Create Secret Recovery Phrase',
	[StepKey.KeyConfiguration]: 'Configure Validator Keys',
	[StepKey.KeyGeneration]: 'Create Validator Key Files',
	[StepKey.Finish]: 'Finish',
	[StepKey.BTECConfiguration]: 'Configure Withdrawal Address',
	[StepKey.BTECGeneration]: 'Create Credentials Change',
	[StepKey.FinishBTEC]: 'Finish',
	[StepKey.ExitConfiguration]: 'Configure Exit',
	[StepKey.ExitGeneration]: 'Create Exit Transaction',
	[StepKey.FinishExit]: 'Finish',
	[StepKey.PartialDepositConfiguration]: 'Configure Deposit',
	[StepKey.PartialDepositGeneration]: 'Create Deposit Data',
	[StepKey.FinishPartialDeposit]: 'Finish',
};

export const CreateMnemonicFlow = [StepKey.MnemonicGeneration, StepKey.KeyConfiguration, StepKey.KeyGeneration, StepKey.Finish];
export const ExistingMnemonicFlow = [StepKey.MnemonicImport, StepKey.KeyConfiguration, StepKey.KeyGeneration, StepKey.Finish];
export const BTECFlow = [StepKey.MnemonicImport, StepKey.BTECConfiguration, StepKey.BTECGeneration, StepKey.FinishBTEC];
export const BTECKeystoreFlow = [StepKey.BTECConfiguration, StepKey.BTECGeneration, StepKey.FinishBTEC];
export const ExitMnemonicFlow = [StepKey.MnemonicImport, StepKey.ExitConfiguration, StepKey.ExitGeneration, StepKey.FinishExit];
export const ExitKeystoreFlow = [StepKey.ExitConfiguration, StepKey.ExitGeneration, StepKey.FinishExit];
export const PartialDepositFlow = [StepKey.PartialDepositConfiguration, StepKey.PartialDepositGeneration, StepKey.FinishPartialDeposit];

export const paths = {
	CREATE_MNEMONIC: "/create",
	CONFIGURE_CREATE: "/configure-create",
	CREATE_KEYS_CREATE: "/create-keys",
	FINISH_CREATE:"/finish-create",

	EXISTING_IMPORT: "/import-existing",
	CONFIGURE_EXISTING: "/configure-existing",
	CREATE_KEYS_EXISTING: "/create-existing-keys",
	FINISH_EXISTING: "/finish-existing",

	BTEC_IMPORT: "/import-btec",
	CONFIGURE_BTEC: "/configure-btec",
	CONFIGURE_BTEC_KEYSTORE: "/configure-btec-keystore",
	CREATE_CREDENTIALS: "/create-btec",
	FINISH_CREDENTIALS: "/finish-btec",

	EXIT_IMPORT: "/import-exit",
	CONFIGURE_EXIT: "/configure-exit",
	CONFIGURE_EXIT_KEYSTORE: "/configure-exit-keystore",
	CREATE_EXIT: "/create-exit",
	FINISH_EXIT: "/finish-exit",

	CONFIGURE_PARTIAL_DEPOSIT: "/configure-partial-deposit",
	CREATE_PARTIAL_DEPOSIT: "/create-partial-deposit",
	FINISH_PARTIAL_DEPOSIT: "/finish-partial-deposit",
};
