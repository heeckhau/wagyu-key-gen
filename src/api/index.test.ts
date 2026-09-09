import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";

import { bashUtils, eth2Deposit, web3Utils } from "./index";

interface Call {
  cmd: string;
  args: unknown;
}

function record(responses: Record<string, unknown> = {}): Call[] {
  const calls: Call[] = [];
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    return responses[cmd];
  });
  return calls;
}

afterEach(() => {
  clearMocks();
});

describe("Tauri bridge", () => {
  it("sends generate_keys arguments as one request object with the amount as a string", async () => {
    const result = { keystore_files: ["/keys/keystore-0.json"], deposit_data_file: "/keys/deposit_data-1.json", pubkeys: ["ab"] };
    const calls = record({ generate_keys: result });

    await expect(
      eth2Deposit.generateKeys("legal winner", 3, "1.5", 2, "Gnosis", "MyPasswordIs", "0x00", true, "/keys"),
    ).resolves.toEqual(result);

    expect(calls).toEqual([
      {
        cmd: "generate_keys",
        args: {
          request: {
            mnemonic: "legal winner",
            index: 3,
            amount: "1.5",
            count: 2,
            network: "Gnosis",
            password: "MyPasswordIs",
            withdrawalAddress: "0x00",
            compounding: true,
            folder: "/keys",
            kdf: "scrypt",
          },
        },
      },
    ]);
  });

  it("maps the remaining commands one to one", async () => {
    const calls = record({
      create_mnemonic: "abandon about",
      validate_mnemonic: "abandon about",
      generate_bls_change: "/out/bls_to_execution_change-1.json",
      is_address: true,
      does_directory_exist: true,
      is_directory_writable: false,
      find_first_file: "",
    });

    await expect(eth2Deposit.createMnemonic("english")).resolves.toBe("abandon about");
    await expect(eth2Deposit.validateMnemonic("aban abou")).resolves.toBe("abandon about");
    await expect(eth2Deposit.validateBLSCredentials("Mainnet", "m", 0, "0x00ab")).resolves.toBeUndefined();
    await expect(eth2Deposit.generateBLSChange("/out", "Mainnet", "m", 0, "1,2", "0x00ab,0x00cd", "0x34")).resolves.toBe(
      "/out/bls_to_execution_change-1.json",
    );
    await expect(web3Utils.isAddress("0x34")).resolves.toBe(true);
    await expect(bashUtils.doesDirectoryExist("/out")).resolves.toBe(true);
    await expect(bashUtils.isDirectoryWritable("/out")).resolves.toBe(false);
    await expect(bashUtils.findFirstFile("/out", "keystore")).resolves.toBe("");

    expect(calls.map((c) => c.cmd)).toEqual([
      "create_mnemonic",
      "validate_mnemonic",
      "validate_bls_credentials",
      "generate_bls_change",
      "is_address",
      "does_directory_exist",
      "is_directory_writable",
      "find_first_file",
    ]);
    expect(calls[2].args).toEqual({ chain: "Mainnet", mnemonic: "m", index: 0, withdrawalCredentials: "0x00ab" });
    expect(calls[3].args).toEqual({
      folder: "/out",
      chain: "Mainnet",
      mnemonic: "m",
      index: 0,
      indices: "1,2",
      withdrawalCredentials: "0x00ab,0x00cd",
      executionAddress: "0x34",
    });
    expect(calls[7].args).toEqual({ directory: "/out", startsWith: "keystore" });
  });

  it("rejects with the backend error text so the pages can show it", async () => {
    const message = "That is not a valid mnemonic, please check for typos.";
    mockIPC(() => {
      throw message;
    });
    await expect(eth2Deposit.validateMnemonic("nope")).rejects.toBe(message);
  });
});
