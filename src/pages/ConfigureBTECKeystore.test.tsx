import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";

import { electronAPI, eth2Deposit, web3Utils } from "../api";
import { BTECContext } from "../BTECContext";
import { errors, paths } from "../constants";
import ConfigureBTECKeystore from "./ConfigureBTECKeystore";

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
    setSource: vi.fn(),
    setKeystorePath: vi.fn(),
    setKeystorePassword: vi.fn(),
    setValidatorIndex: vi.fn(),
    setWithdrawalAddress: vi.fn(),
  };
  render(
    <MemoryRouter initialEntries={[paths.CONFIGURE_BTEC_KEYSTORE]}>
      <BTECContext.Provider
        value={{
          source: "mnemonic",
          btecCredentials: "",
          setBTECCredentials: () => {},
          btecIndices: "",
          setBTECIndices: () => {},
          folderLocation: "",
          setFolderLocation: () => {},
          index: 0,
          setIndex: () => {},
          mnemonic: "",
          setMnemonic: () => {},
          withdrawalAddress: "",
          keystorePath: "",
          keystorePassword: "",
          validatorIndex: 0,
          ...spies,
        }}
      >
        <Route path={paths.CONFIGURE_BTEC_KEYSTORE} component={ConfigureBTECKeystore} />
        <Route path={paths.CREATE_CREDENTIALS} render={() => <div>create credentials page</div>} />
      </BTECContext.Provider>
    </MemoryRouter>,
  );
  return spies;
}

function clickNext() {
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
}

afterEach(() => {
  vi.clearAllMocks();
});

describe("ConfigureBTECKeystore", () => {
  it("requires the keystore, password and withdrawal address", async () => {
    const spies = renderPage();
    clickNext();

    expect(await screen.findByText(errors.KEYSTORE_REQUIRED)).toBeInTheDocument();
    expect(screen.getByText(errors.KEYSTORE_PASSWORD_REQUIRED)).toBeInTheDocument();
    expect(screen.getByText(errors.WITHDRAW_ADDRESS_REQUIRED)).toBeInTheDocument();
    expect(spies.setSource).not.toHaveBeenCalled();
  });

  it("stores the details with the keystore source and moves on", async () => {
    const spies = renderPage();
    openDialog.mockResolvedValue(KEYSTORE);
    keystorePubkey.mockResolvedValue("b3e4");
    isAddress.mockResolvedValue(true);
    fireEvent.click(screen.getByText("Select keystore file"));
    await screen.findByText("Validator public key: 0xb3e4");
    fireEvent.change(screen.getByLabelText(/^Keystore password/), { target: { value: "MyPasswordIs" } });
    fireEvent.change(screen.getByLabelText(/^Validator index/), { target: { value: "42" } });
    fireEvent.change(screen.getByLabelText(/^Ethereum Withdrawal Address/), { target: { value: ADDRESS } });
    clickNext();

    expect(await screen.findByText("create credentials page")).toBeInTheDocument();
    expect(spies.setSource).toHaveBeenCalledWith("keystore");
    expect(spies.setKeystorePath).toHaveBeenCalledWith(KEYSTORE);
    expect(spies.setKeystorePassword).toHaveBeenCalledWith("MyPasswordIs");
    expect(spies.setValidatorIndex).toHaveBeenCalledWith(42);
    expect(spies.setWithdrawalAddress).toHaveBeenCalledWith(ADDRESS);
  });
});
