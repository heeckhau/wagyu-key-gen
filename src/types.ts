export enum StepKey {
  MnemonicImport,
  MnemonicGeneration,
  KeyConfiguration,
  KeyGeneration,
  Finish,
  BTECConfiguration,
  BTECGeneration,
  FinishBTEC,
  ExitConfiguration,
  ExitGeneration,
  FinishExit,
  PartialDepositConfiguration,
  PartialDepositGeneration,
  FinishPartialDeposit,
}

export enum StepSequenceKey {
  MnemonicGeneration = "mnemonicgeneration",
  MnemonicImport = "mnemonicimport",
  BLSToExecutionChangeGeneration = "blstoexecutionchangegeneration",
}

export enum ReuseMnemonicAction {
  RegenerateKeys,
  GenerateBLSToExecutionChange,
  GenerateExitTransaction,
}

/** What the user wants to do with an existing keystore file. */
export enum KeystoreAction {
  GenerateExitTransaction,
  GeneratePartialDeposit,
  GenerateBLSToExecutionChange,
}

/** Whether a keystore-capable flow was started from a mnemonic or from a keystore file. */
export type KeySource = "mnemonic" | "keystore";

export enum Network {
  MAINNET = "Mainnet",
  HOODI = "Hoodi",
  GNOSIS = "Gnosis",
  CHIADO = "Chiado",
}

export interface NetworkConfig {
  multiplier: number;
}

export const NetworkConfig: Record<Network, NetworkConfig> = {
  [Network.MAINNET]: {
    multiplier: 1,
  },
  [Network.HOODI]: {
    multiplier: 1,
  },
  [Network.GNOSIS]: {
    multiplier: 32,
  },
  [Network.CHIADO]: {
    multiplier: 32,
  },
};