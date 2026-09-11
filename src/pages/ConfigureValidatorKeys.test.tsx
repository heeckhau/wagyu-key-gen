import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";

import { web3Utils } from "../api";
import { errors, formatDepositAmountError, paths } from "../constants";
import { KeyCreationContext } from "../KeyCreationContext";
import { Network } from "../types";
import ConfigureValidatorKeys from "./ConfigureValidatorKeys";

vi.mock("../api", () => ({
  web3Utils: { isAddress: vi.fn() },
}));

const isAddress = vi.mocked(web3Utils.isAddress);

const MNEMONIC =
  "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const VALID_PASSWORD = "StrongPass!!";
const ADDRESS = "0x000000000000000000000000000000000000aa";

function renderPage(overrides: Record<string, unknown> = {}) {
  const spies = {
    setIndex: vi.fn(),
    setNumberOfKeys: vi.fn(),
    setAmount: vi.fn(),
    setPassword: vi.fn(),
    setWithdrawalAddress: vi.fn(),
    setCompounding: vi.fn(),
  };
  const value = {
    folderLocation: "",
    setFolderLocation: () => {},
    index: 0,
    setIndex: spies.setIndex,
    mnemonic: MNEMONIC,
    setMnemonic: () => {},
    numberOfKeys: 1,
    setNumberOfKeys: spies.setNumberOfKeys,
    amount: 32,
    setAmount: spies.setAmount,
    password: "",
    setPassword: spies.setPassword,
    withdrawalAddress: "",
    setWithdrawalAddress: spies.setWithdrawalAddress,
    compounding: false,
    setCompounding: spies.setCompounding,
    ...overrides,
  };

  // The CONFIGURE_CREATE path drives the new-keys flow (no starting-index field, no
  // "existing" wording); ConfigureValidatorKeys.test covers the CONFIGURE_EXISTING branch
  // only through the parts of validateInputs that don't depend on which flow it is.
  render(
    <MemoryRouter initialEntries={[paths.CONFIGURE_CREATE]}>
      <KeyCreationContext.Provider value={value}>
        <Route path={paths.CONFIGURE_CREATE} component={ConfigureValidatorKeys} />
        <Route path={paths.CREATE_KEYS_CREATE} render={() => <div>create keys page</div>} />
      </KeyCreationContext.Provider>
    </MemoryRouter>,
  );

  return spies;
}

function fillBaseForm() {
  fireEvent.change(screen.getByLabelText("Password"), { target: { value: VALID_PASSWORD } });
}

function setWithdrawalAddress(address: string) {
  fireEvent.change(screen.getByLabelText("Ethereum Withdrawal Address (Optional)"), {
    target: { value: address },
  });
}

function toggleCompounding() {
  fireEvent.click(screen.getByRole("checkbox"));
}

function clickNext() {
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
}

afterEach(() => {
  vi.clearAllMocks();
});

describe("ConfigureValidatorKeys", () => {
  it("rejects a key count outside 1-1000", async () => {
    renderPage();
    fillBaseForm();
    fireEvent.change(screen.getByLabelText("Number of New Keys"), { target: { value: "1001" } });
    clickNext();

    expect(await screen.findByText(errors.NUMBER_OF_KEYS)).toBeInTheDocument();
    expect(screen.queryByLabelText("Retype Password")).not.toBeInTheDocument();
  });

  it("enforces a minimum password length", async () => {
    renderPage();
    fireEvent.change(screen.getByLabelText("Password"), { target: { value: "short" } });
    clickNext();

    expect(await screen.findByText(errors.PASSWORD_STRENGTH)).toBeInTheDocument();
  });

  it("rejects an invalid withdrawal address", async () => {
    isAddress.mockResolvedValue(false);
    renderPage();
    fillBaseForm();
    setWithdrawalAddress("not-an-address");
    clickNext();

    expect(await screen.findByText(errors.ADDRESS_FORMAT_ERROR)).toBeInTheDocument();
  });

  it("enforces deposit-amount limits when compounding is on", async () => {
    isAddress.mockResolvedValue(true);
    renderPage();
    fillBaseForm();
    setWithdrawalAddress(ADDRESS);
    toggleCompounding();
    fireEvent.change(screen.getByLabelText("Deposit Amount"), { target: { value: "5000" } });
    clickNext();

    expect(await screen.findByText(formatDepositAmountError(Network.MAINNET))).toBeInTheDocument();
  });

  it("resets the deposit amount and disables compounding when the address is cleared", () => {
    renderPage();
    setWithdrawalAddress(ADDRESS);
    toggleCompounding();
    fireEvent.change(screen.getByLabelText("Deposit Amount"), { target: { value: "500" } });
    setWithdrawalAddress("");

    expect(screen.getByRole("checkbox")).not.toBeChecked();
    expect(screen.getByLabelText("Deposit Amount")).toHaveValue(32);
  });

  it("resets the deposit amount when compounding is turned back off", () => {
    renderPage();
    setWithdrawalAddress(ADDRESS);
    toggleCompounding();
    fireEvent.change(screen.getByLabelText("Deposit Amount"), { target: { value: "500" } });
    toggleCompounding();

    expect(screen.getByLabelText("Deposit Amount")).toHaveValue(32);
  });

  it("collects the form, verifies the retyped password, and commits everything to context", async () => {
    const spies = renderPage();
    fireEvent.change(screen.getByLabelText("Number of New Keys"), { target: { value: "3" } });
    fillBaseForm();
    clickNext();

    fireEvent.change(await screen.findByLabelText("Retype Password"), { target: { value: VALID_PASSWORD } });
    clickNext();

    expect(await screen.findByText("create keys page")).toBeInTheDocument();
    expect(spies.setIndex).toHaveBeenCalledWith(0);
    expect(spies.setNumberOfKeys).toHaveBeenCalledWith(3);
    expect(spies.setAmount).toHaveBeenCalledWith(32);
    expect(spies.setPassword).toHaveBeenCalledWith(VALID_PASSWORD);
    expect(spies.setWithdrawalAddress).toHaveBeenCalledWith("");
    expect(spies.setCompounding).toHaveBeenCalledWith(false);
  });

  it("flags mismatched retyped passwords without committing", async () => {
    const spies = renderPage();
    fillBaseForm();
    clickNext();

    fireEvent.change(await screen.findByLabelText("Retype Password"), { target: { value: "SomethingElse!!" } });
    clickNext();

    expect(await screen.findByText(errors.PASSWORD_MATCH)).toBeInTheDocument();
    expect(screen.queryByText("create keys page")).not.toBeInTheDocument();
    expect(spies.setPassword).not.toHaveBeenCalled();
  });

  it("resets the context back to defaults when going back from the form", () => {
    const spies = renderPage();
    fireEvent.click(screen.getByRole("button", { name: "Back" }));

    expect(spies.setIndex).toHaveBeenCalledWith(0);
    expect(spies.setNumberOfKeys).toHaveBeenCalledWith(1);
    expect(spies.setAmount).toHaveBeenCalledWith(32);
    expect(spies.setWithdrawalAddress).toHaveBeenCalledWith("");
    expect(spies.setPassword).toHaveBeenCalledWith("");
    expect(spies.setCompounding).toHaveBeenCalledWith(false);
  });
});
