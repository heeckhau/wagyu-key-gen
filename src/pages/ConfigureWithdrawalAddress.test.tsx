import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";

import { eth2Deposit, web3Utils } from "../api";
import { BTECContext } from "../BTECContext";
import { errors, paths } from "../constants";
import ConfigureWithdrawalAddress from "./ConfigureWithdrawalAddress";

vi.mock("../api", () => ({
  eth2Deposit: { validateBLSCredentials: vi.fn() },
  web3Utils: { isAddress: vi.fn() },
}));

const validateBLSCredentials = vi.mocked(eth2Deposit.validateBLSCredentials);
const isAddress = vi.mocked(web3Utils.isAddress);

const MNEMONIC =
  "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
// A real 0x00 BLS withdrawal credential: 0x + the 0x00 prefix + 62 hex characters.
const VALID_CRED = "0x00" + "ab".repeat(31);

function renderPage(overrides: Record<string, unknown> = {}) {
  const spies = {
    setBTECCredentials: vi.fn(),
    setBTECIndices: vi.fn(),
    setIndex: vi.fn(),
    setWithdrawalAddress: vi.fn(),
  };
  const value = {
    btecCredentials: "",
    setBTECCredentials: spies.setBTECCredentials,
    btecIndices: "",
    setBTECIndices: spies.setBTECIndices,
    folderLocation: "",
    setFolderLocation: () => {},
    index: 0,
    setIndex: spies.setIndex,
    mnemonic: MNEMONIC,
    setMnemonic: () => {},
    withdrawalAddress: "",
    setWithdrawalAddress: spies.setWithdrawalAddress,
    ...overrides,
  };

  render(
    <MemoryRouter initialEntries={[paths.CONFIGURE_BTEC]}>
      <BTECContext.Provider value={value}>
        <Route path={paths.CONFIGURE_BTEC} component={ConfigureWithdrawalAddress} />
        <Route path={paths.CREATE_CREDENTIALS} render={() => <div>create credentials page</div>} />
      </BTECContext.Provider>
    </MemoryRouter>,
  );

  return spies;
}

function fillForm({
  indices = "0",
  credentials = VALID_CRED,
  address = "0x000000000000000000000000000000000000aa",
} = {}) {
  // These fields are all `required`, so MUI appends a "*" to the accessible label text;
  // anchor on the prefix instead of matching it exactly.
  fireEvent.change(screen.getByLabelText(/^Indices or validator indexes \(comma separated\)/), {
    target: { value: indices },
  });
  fireEvent.change(screen.getByLabelText(/^BLS withdrawal credentials \(comma separated\)/), {
    target: { value: credentials },
  });
  fireEvent.change(screen.getByLabelText(/^Ethereum Withdrawal Address/), { target: { value: address } });
}

function clickNext() {
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
}

afterEach(() => {
  vi.clearAllMocks();
});

describe("ConfigureWithdrawalAddress", () => {
  it("rejects indices that aren't plain digits", async () => {
    isAddress.mockResolvedValue(true);
    renderPage();
    fillForm({ indices: "1,x", credentials: `${VALID_CRED},${VALID_CRED}` });
    clickNext();

    expect(await screen.findByText(errors.INDICES_FORMAT)).toBeInTheDocument();
    expect(validateBLSCredentials).not.toHaveBeenCalled();
  });

  it("rejects a badly formatted BLS credential", async () => {
    isAddress.mockResolvedValue(true);
    renderPage();
    fillForm({ indices: "1,2", credentials: `${VALID_CRED},not-a-credential` });
    clickNext();

    expect(await screen.findByText(errors.BLS_CREDENTIALS_FORMAT)).toBeInTheDocument();
    expect(validateBLSCredentials).not.toHaveBeenCalled();
  });

  it("rejects mismatched indices/credentials counts", async () => {
    isAddress.mockResolvedValue(true);
    renderPage();
    fillForm({ indices: "1,2,3", credentials: VALID_CRED });
    clickNext();

    expect(await screen.findByText(errors.INDICES_LENGTH)).toBeInTheDocument();
    expect(validateBLSCredentials).not.toHaveBeenCalled();
  });

  it("requires a withdrawal address, unlike the new-keys flow where it's optional", async () => {
    renderPage();
    fillForm({ address: "" });
    clickNext();

    expect(await screen.findByText(errors.WITHDRAW_ADDRESS_REQUIRED)).toBeInTheDocument();
    expect(validateBLSCredentials).not.toHaveBeenCalled();
  });

  it("rejects an invalid withdrawal address", async () => {
    isAddress.mockResolvedValue(false);
    renderPage();
    fillForm({ address: "not-an-address" });
    clickNext();

    expect(await screen.findByText(errors.ADDRESS_FORMAT_ERROR)).toBeInTheDocument();
    expect(validateBLSCredentials).not.toHaveBeenCalled();
  });

  it("validates the credentials and advances once everything checks out", async () => {
    isAddress.mockResolvedValue(true);
    validateBLSCredentials.mockResolvedValue(undefined);
    const spies = renderPage();
    fillForm({ indices: "0", credentials: VALID_CRED, address: "0x000000000000000000000000000000000000aa" });
    clickNext();

    expect(await screen.findByText("create credentials page")).toBeInTheDocument();
    expect(validateBLSCredentials).toHaveBeenCalledWith("Mainnet", MNEMONIC, 0, VALID_CRED);
    expect(spies.setBTECCredentials).toHaveBeenCalledWith(VALID_CRED);
    expect(spies.setBTECIndices).toHaveBeenCalledWith("0");
    expect(spies.setIndex).toHaveBeenCalledWith(0);
    expect(spies.setWithdrawalAddress).toHaveBeenCalledWith("0x000000000000000000000000000000000000aa");
  });

  it("surfaces a no-match error when the backend rejects the credentials", async () => {
    isAddress.mockResolvedValue(true);
    validateBLSCredentials.mockRejectedValue(new Error("no match"));
    renderPage();
    fillForm();
    clickNext();

    expect(await screen.findByText(errors.BLS_CREDENTIALS_NO_MATCH)).toBeInTheDocument();
    expect(screen.queryByText("create credentials page")).not.toBeInTheDocument();
  });

  it("resets the form and the shared context when going back", () => {
    const spies = renderPage();
    fireEvent.click(screen.getByRole("button", { name: "Back" }));

    expect(spies.setBTECCredentials).toHaveBeenCalledWith("");
    expect(spies.setBTECIndices).toHaveBeenCalledWith("");
    expect(spies.setIndex).toHaveBeenCalledWith(0);
    expect(spies.setWithdrawalAddress).toHaveBeenCalledWith("");
  });
});
