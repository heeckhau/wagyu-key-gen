/**
 * Bridge between the React UI and the Rust backend.
 *
 * This replaces the Electron preload bridge (`window.electronAPI`, `window.eth2Deposit`,
 * `window.bashUtils`, `window.web3Utils`). The method names and signatures are kept so the pages
 * only had to swap the `window.` prefix for an import. Every call goes through Tauri's `invoke`
 * or one of its plugins; errors are rejected as plain strings.
 */
import { invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

export interface VersionInfo {
  version: string;
  commit: string;
}

export interface GenerateKeysResult {
  keystore_files: string[];
  deposit_data_file: string;
  pubkeys: string[];
}

/** Keystore key derivation function. `scrypt` is the deposit-cli default. */
export type KdfChoice = "scrypt" | "pbkdf2";

export const electronAPI = {
  clipboardWriteText: (text: string): Promise<void> => writeText(text),

  /** Opens the native folder picker. Resolves to the chosen folder or `null` when cancelled. */
  invokeShowOpenDialog: async (): Promise<string | null> => {
    const selected = await open({ directory: true, multiple: false });
    return typeof selected === "string" ? selected : null;
  },

  /** Quits the application (the backend clears the clipboard on exit). */
  ipcRendererSendClose: (): Promise<void> => invoke("quit"),

  /** Reveals a file in the OS file manager. */
  shellShowItemInFolder: (fullPath: string): Promise<void> => revealItemInDir(fullPath),

  versionInfo: (): Promise<VersionInfo> => invoke("version_info"),
};

export const eth2Deposit = {
  createMnemonic: (language: string): Promise<string> => invoke("create_mnemonic", { language }),

  /**
   * Validates a mnemonic (abbreviated words and any BIP-39 language accepted) and resolves to its
   * full-word form. Rejects with a message when the mnemonic is invalid.
   */
  validateMnemonic: (mnemonic: string): Promise<string> => invoke("validate_mnemonic", { mnemonic }),

  /**
   * Generates keystores and deposit data.
   *
   * @param amount Deposit per validator as typed by the user, in ETH (or GNO on Gnosis chains).
   *               The backend converts it to gwei exactly and applies the network multiplier.
   */
  generateKeys: (
    mnemonic: string,
    index: number,
    amount: string,
    count: number,
    network: string,
    password: string,
    withdrawalAddress: string,
    compounding: boolean,
    folder: string,
    kdf: KdfChoice = "scrypt",
  ): Promise<GenerateKeysResult> =>
    invoke("generate_keys", {
      request: { mnemonic, index, amount, count, network, password, withdrawalAddress, compounding, folder, kdf },
    }),

  validateBLSCredentials: (
    chain: string,
    mnemonic: string,
    index: number,
    withdrawalCredentials: string,
  ): Promise<void> => invoke("validate_bls_credentials", { chain, mnemonic, index, withdrawalCredentials }),

  /** Writes the BLS-to-execution-change file and resolves to its path. */
  generateBLSChange: (
    folder: string,
    chain: string,
    mnemonic: string,
    index: number,
    indices: string,
    withdrawalCredentials: string,
    executionAddress: string,
  ): Promise<string> =>
    invoke("generate_bls_change", {
      folder,
      chain,
      mnemonic,
      index,
      indices,
      withdrawalCredentials,
      executionAddress,
    }),
};

export const bashUtils = {
  doesDirectoryExist: (directory: string): Promise<boolean> => invoke("does_directory_exist", { directory }),
  isDirectoryWritable: (directory: string): Promise<boolean> => invoke("is_directory_writable", { directory }),
  /** Resolves to the first file in `directory` whose name starts with `startsWith`, or `""`. */
  findFirstFile: (directory: string, startsWith: string): Promise<string> =>
    invoke("find_first_file", { directory, startsWith }),
};

export const web3Utils = {
  isAddress: (address: string): Promise<boolean> => invoke("is_address", { address }),
};
