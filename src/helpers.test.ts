import { describe, expect, it } from "vitest";

import { cleanMnemonic } from "./helpers";

describe("cleanMnemonic", () => {
  it("replaces punctuation with spaces and collapses whitespace", () => {
    expect(cleanMnemonic("  abandon,  abandon.abandon\n abandon ")).toBe("abandon abandon abandon abandon");
  });

  it("keeps non-ASCII words untouched", () => {
    expect(cleanMnemonic("的 的 在")).toBe("的 的 在");
  });

  it("leaves a clean mnemonic unchanged", () => {
    const mnemonic = "legal winner thank year wave sausage worth useful legal winner thank yellow";
    expect(cleanMnemonic(mnemonic)).toBe(mnemonic);
  });
});
