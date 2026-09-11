import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { GlobalContext } from "../GlobalContext";
import { KeyCreationContext } from "../KeyCreationContext";
import { Network } from "../types";
import VerifyMnemonic from "./VerifyMnemonic";

const noop = () => {};

const baseKeyCreationContext = {
  folderLocation: "",
  setFolderLocation: noop,
  index: 0,
  setIndex: noop,
  mnemonic: "",
  setMnemonic: noop,
  numberOfKeys: 1,
  setNumberOfKeys: noop,
  amount: 32,
  setAmount: noop,
  password: "",
  setPassword: noop,
  withdrawalAddress: "",
  setWithdrawalAddress: noop,
  compounding: false,
  setCompounding: noop,
};

function renderVerify({ mnemonic = "", mnemonicToVerify = "", hasError = false } = {}) {
  const setMnemonicToVerify = vi.fn();
  const onVerifyMnemonic = vi.fn();

  render(
    <GlobalContext.Provider value={{ network: Network.MAINNET, setNetwork: noop }}>
      <KeyCreationContext.Provider value={{ ...baseKeyCreationContext, mnemonic }}>
        <VerifyMnemonic
          hasError={hasError}
          mnemonicToVerify={mnemonicToVerify}
          setMnemonicToVerify={setMnemonicToVerify}
          onVerifyMnemonic={onVerifyMnemonic}
        />
      </KeyCreationContext.Provider>
    </GlobalContext.Provider>,
  );

  return { setMnemonicToVerify, onVerifyMnemonic };
}

// On mainnet/gnosis the phrase is entered as a grid of 24 per-word boxes.
describe("VerifyMnemonic (mainnet grid)", () => {
  it("joins the 24 words back together as the user types", () => {
    const { setMnemonicToVerify } = renderVerify();
    fireEvent.change(screen.getByLabelText("Word 1"), { target: { value: "abandon" } });

    const expected = Array(24).fill("");
    expected[0] = "abandon";
    expect(setMnemonicToVerify).toHaveBeenLastCalledWith(expected.join(" "));
  });

  it("strips whitespace typed inside a word", () => {
    const { setMnemonicToVerify } = renderVerify();
    fireEvent.change(screen.getByLabelText("Word 1"), { target: { value: "aban don" } });

    const expected = Array(24).fill("");
    expected[0] = "abandon";
    expect(setMnemonicToVerify).toHaveBeenLastCalledWith(expected.join(" "));
  });

  it("advances focus to the next word after a space once 3+ letters are typed", () => {
    renderVerify();
    const word1 = screen.getByLabelText("Word 1");
    fireEvent.change(word1, { target: { value: "aban" } });
    fireEvent.keyDown(word1, { key: " " });

    expect(document.activeElement).toBe(screen.getByLabelText("Word 2"));
  });

  it("does not jump ahead on a space before 3 letters are typed", () => {
    renderVerify();
    const word1 = screen.getByLabelText("Word 1");
    fireEvent.change(word1, { target: { value: "ab" } });
    fireEvent.keyDown(word1, { key: " " });

    expect(document.activeElement).toBe(word1);
  });

  it("backspacing an empty word moves focus to the previous one", () => {
    renderVerify();
    const word2 = screen.getByLabelText("Word 2");
    fireEvent.keyDown(word2, { key: "Backspace" });

    expect(document.activeElement).toBe(screen.getByLabelText("Word 1"));
  });

  it.each([
    ["ArrowRight", "Word 2"],
    ["ArrowLeft", "Word 24"],
    ["ArrowDown", "Word 7"],
    ["ArrowUp", "Word 19"],
  ])("%s from word 1 focuses %s", (key, label) => {
    renderVerify();
    const word1 = screen.getByLabelText("Word 1");
    fireEvent.keyDown(word1, { key });

    expect(document.activeElement).toBe(screen.getByLabelText(label));
  });

  it("submits on Enter at the last word", () => {
    const { onVerifyMnemonic } = renderVerify();
    const word24 = screen.getByLabelText("Word 24");
    fireEvent.change(word24, { target: { value: "zebra" } });
    fireEvent.keyDown(word24, { key: "Enter" });

    expect(onVerifyMnemonic).toHaveBeenCalledTimes(1);
  });

  it("flags only the word that doesn't match the real mnemonic", () => {
    const words = Array.from({ length: 24 }, (_, i) => `word${i + 1}`);
    const correct = words.join(" ");
    const typed = [...words];
    typed[5] = "oops";

    renderVerify({ mnemonic: correct, mnemonicToVerify: typed.join(" "), hasError: true });

    expect(screen.getByLabelText("Word 6")).toBeInvalid();
    expect(screen.getByLabelText("Word 1")).not.toBeInvalid();
  });
});
