import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";

import { bashUtils, electronAPI, eth2Deposit } from "../api";
import { paths } from "../constants";
import { ExitContext } from "../ExitContext";
import { GlobalContext } from "../GlobalContext";
import { exitContextValue } from "../testing/exitContext";
import { Network } from "../types";
import CreateExitTransaction from "./CreateExitTransaction";

vi.mock("../api", () => ({
  bashUtils: { doesDirectoryExist: vi.fn(), isDirectoryWritable: vi.fn() },
  electronAPI: { invokeShowOpenDialog: vi.fn() },
  eth2Deposit: { generateExitTransactions: vi.fn(), generateExitTransactionKeystore: vi.fn() },
}));

const generateExitTransactions = vi.mocked(eth2Deposit.generateExitTransactions);
const generateExitTransactionKeystore = vi.mocked(eth2Deposit.generateExitTransactionKeystore);

const MNEMONIC =
  "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const KEYSTORE = "/keys/keystore-m_12381_3600_0_0_0-1.json";

function renderPage(overrides: Record<string, unknown>) {
  const { value, spies } = exitContextValue(overrides);
  render(
    <MemoryRouter initialEntries={[paths.CREATE_EXIT]}>
      <GlobalContext.Provider value={{ network: Network.HOODI, setNetwork: () => {} }}>
        <ExitContext.Provider value={value}>
          <Route path={paths.CREATE_EXIT} component={CreateExitTransaction} />
          <Route path={paths.FINISH_EXIT} render={() => <div>finish page</div>} />
          <Route path={paths.EXIT_IMPORT} render={() => <div>import page</div>} />
          <Route path={paths.CONFIGURE_EXIT_KEYSTORE} render={() => <div>configure keystore page</div>} />
        </ExitContext.Provider>
      </GlobalContext.Provider>
    </MemoryRouter>,
  );
  return spies;
}

async function chooseFolderAndCreate() {
  vi.mocked(electronAPI.invokeShowOpenDialog).mockResolvedValue("/out");
  vi.mocked(bashUtils.doesDirectoryExist).mockResolvedValue(true);
  vi.mocked(bashUtils.isDirectoryWritable).mockResolvedValue(true);
  fireEvent.click(screen.getByRole("button", { name: "Browse" }));
  await screen.findByText("You've selected: /out");
  fireEvent.click(screen.getByRole("button", { name: "Create" }));
}

afterEach(() => {
  vi.clearAllMocks();
});

describe("CreateExitTransaction", () => {
  it("signs with the mnemonic keys and moves to the finish step", async () => {
    generateExitTransactions.mockResolvedValue(["/out/signed_exit_transaction-1-1.json"]);
    const spies = renderPage({ source: "mnemonic", mnemonic: MNEMONIC, index: 3, indices: "1,2", epoch: 1234 });
    await chooseFolderAndCreate();

    expect(await screen.findByText("finish page")).toBeInTheDocument();
    expect(generateExitTransactions).toHaveBeenCalledWith("/out", Network.HOODI, MNEMONIC, 3, "1,2", 1234);
    expect(generateExitTransactionKeystore).not.toHaveBeenCalled();
    expect(spies.setFolderLocation).toHaveBeenCalledWith("/out");
  });

  it("signs with the keystore and moves to the finish step", async () => {
    generateExitTransactionKeystore.mockResolvedValue("/out/signed_exit_transaction-7-1.json");
    renderPage({ source: "keystore", keystorePath: KEYSTORE, keystorePassword: "MyPasswordIs", validatorIndex: 7, epoch: 0 });
    await chooseFolderAndCreate();

    expect(await screen.findByText("finish page")).toBeInTheDocument();
    expect(generateExitTransactionKeystore).toHaveBeenCalledWith("/out", Network.HOODI, KEYSTORE, "MyPasswordIs", 7, 0);
    expect(generateExitTransactions).not.toHaveBeenCalled();
  });

  it("shows the backend error and lets the user try again", async () => {
    generateExitTransactionKeystore.mockRejectedValue("The keystore password is incorrect.");
    renderPage({ source: "keystore", keystorePath: KEYSTORE, keystorePassword: "wrong", validatorIndex: 7 });
    await chooseFolderAndCreate();

    expect(await screen.findByText("The keystore password is incorrect.")).toBeInTheDocument();
    expect(screen.queryByText("finish page")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Back" })).toBeInTheDocument();
  });

  it("goes back to the right configuration step when the inputs are missing", () => {
    renderPage({ source: "keystore", keystorePath: "" });
    expect(screen.getByText("configure keystore page")).toBeInTheDocument();
  });
});
