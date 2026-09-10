import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";

import { electronAPI, eth2Deposit, web3Utils } from "../api";
import { errors, formatDepositAmountError, paths } from "../constants";
import { PartialDepositContext } from "../PartialDepositContext";
import { Network } from "../types";
import ConfigurePartialDeposit from "./ConfigurePartialDeposit";

vi.mock("../api", () => ({
  electronAPI: { invokeShowOpenKeystoreDialog: vi.fn() },
  eth2Deposit: { keystorePubkey: vi.fn() },
  web3Utils: { isAddress: vi.fn() },
}));

const openDialog = vi.mocked(electronAPI.invokeShowOpenKeystoreDialog);
const keystorePubkey = vi.mocked(eth2Deposit.keystorePubkey);
const isAddress = vi.mocked(web3Utils.isAddress);

const KEYSTORE = "/keys/keystore-m_12381_3600_0_0_0-1.json";
const ADDRESS = "0x000000000000000000000000000000000000aa";

function renderPage() {
  const spies = {
    setKeystorePath: vi.fn(),
    setKeystorePassword: vi.fn(),
    setAmount: vi.fn(),
    setWithdrawalAddress: vi.fn(),
    setCompounding: vi.fn(),
    setFolderLocation: vi.fn(),
  };
  render(
    <MemoryRouter initialEntries={[paths.CONFIGURE_PARTIAL_DEPOSIT]}>
      <PartialDepositContext.Provider
        value={{
          keystorePath: "",
          keystorePassword: "",
          amount: "",
          withdrawalAddress: "",
          compounding: false,
          folderLocation: "",
          ...spies,
        }}
      >
        <Route path={paths.CONFIGURE_PARTIAL_DEPOSIT} component={ConfigurePartialDeposit} />
        <Route path={paths.CREATE_PARTIAL_DEPOSIT} render={() => <div>create deposit page</div>} />
      </PartialDepositContext.Provider>
    </MemoryRouter>,
  );
  return spies;
}

async function fillValidForm() {
  openDialog.mockResolvedValue(KEYSTORE);
  keystorePubkey.mockResolvedValue("b3e4");
  isAddress.mockResolvedValue(true);
  fireEvent.click(screen.getByText("Select keystore file"));
  await screen.findByText("Validator public key: 0xb3e4");
  fireEvent.change(screen.getByLabelText(/^Keystore password/), { target: { value: "MyPasswordIs" } });
  fireEvent.change(screen.getByLabelText(/^Deposit Amount/), { target: { value: "1.5" } });
  fireEvent.change(screen.getByLabelText(/^Ethereum Withdrawal Address/), { target: { value: ADDRESS } });
}

function clickNext() {
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
}

afterEach(() => {
  vi.clearAllMocks();
});

describe("ConfigurePartialDeposit", () => {
  it("requires every field", async () => {
    const spies = renderPage();
    clickNext();

    expect(await screen.findByText(errors.KEYSTORE_REQUIRED)).toBeInTheDocument();
    expect(screen.getByText(errors.KEYSTORE_PASSWORD_REQUIRED)).toBeInTheDocument();
    expect(screen.getByText(formatDepositAmountError(Network.MAINNET))).toBeInTheDocument();
    expect(screen.getByText(errors.WITHDRAW_ADDRESS_REQUIRED)).toBeInTheDocument();
    expect(spies.setKeystorePath).not.toHaveBeenCalled();
  });

  it("enforces the deposit amount limits and trims to 1 gwei precision", async () => {
    renderPage();
    await fillValidForm();
    fireEvent.change(screen.getByLabelText(/^Deposit Amount/), { target: { value: "2049" } });
    clickNext();
    expect(await screen.findByText(formatDepositAmountError(Network.MAINNET))).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText(/^Deposit Amount/), { target: { value: "1.1234567891234" } });
    expect(screen.getByLabelText(/^Deposit Amount/)).toHaveValue(1.123456789);
  });

  it("rejects an invalid withdrawal address", async () => {
    renderPage();
    await fillValidForm();
    isAddress.mockResolvedValue(false);
    fireEvent.change(screen.getByLabelText(/^Ethereum Withdrawal Address/), { target: { value: "nope" } });
    clickNext();

    expect(await screen.findByText(errors.ADDRESS_FORMAT_ERROR)).toBeInTheDocument();
    expect(screen.queryByText("create deposit page")).not.toBeInTheDocument();
  });

  it("stores the deposit details and moves on", async () => {
    const spies = renderPage();
    await fillValidForm();
    fireEvent.click(screen.getByRole("checkbox"));
    clickNext();

    expect(await screen.findByText("create deposit page")).toBeInTheDocument();
    expect(spies.setKeystorePath).toHaveBeenCalledWith(KEYSTORE);
    expect(spies.setKeystorePassword).toHaveBeenCalledWith("MyPasswordIs");
    expect(spies.setAmount).toHaveBeenCalledWith("1.5");
    expect(spies.setWithdrawalAddress).toHaveBeenCalledWith(ADDRESS);
    expect(spies.setCompounding).toHaveBeenCalledWith(true);
  });
});
