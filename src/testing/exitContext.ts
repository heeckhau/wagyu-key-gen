import { vi } from "vitest";

import { KeySource } from "../types";

/** A full ExitContext value with spies for every setter, for rendering exit pages in isolation. */
export function exitContextValue(overrides: Record<string, unknown> = {}) {
  const spies = {
    setSource: vi.fn(),
    setMnemonic: vi.fn(),
    setIndex: vi.fn(),
    setIndices: vi.fn(),
    setKeystorePath: vi.fn(),
    setKeystorePassword: vi.fn(),
    setValidatorIndex: vi.fn(),
    setEpoch: vi.fn(),
    setFolderLocation: vi.fn(),
  };
  const value = {
    source: "mnemonic" as KeySource,
    mnemonic: "",
    index: 0,
    indices: "",
    keystorePath: "",
    keystorePassword: "",
    validatorIndex: 0,
    epoch: 0,
    folderLocation: "",
    ...spies,
    ...overrides,
  };
  return { value, spies };
}
