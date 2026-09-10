import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";

import { electronAPI, eth2Deposit } from "../api";
import { errors, paths } from "../constants";
import { ExitContext } from "../ExitContext";
import { exitContextValue } from "../testing/exitContext";
import ConfigureExitKeystore from "./ConfigureExitKeystore";

vi.mock("../api", () => ({
  electronAPI: { invokeShowOpenKeystoreDialog: vi.fn() },
  eth2Deposit: { keystorePubkey: vi.fn() },
}));

const openDialog = vi.mocked(electronAPI.invokeShowOpenKeystoreDialog);
const keystorePubkey = vi.mocked(eth2Deposit.keystorePubkey);

const KEYSTORE = "/keys/keystore-m_12381_3600_0_0_0-1.json";

function renderPage(overrides: Record<string, unknown> = {}) {
  const { value, spies } = exitContextValue(overrides);
  render(
    <MemoryRouter initialEntries={[paths.CONFIGURE_EXIT_KEYSTORE]}>
      <ExitContext.Provider value={value}>
        <Route path={paths.CONFIGURE_EXIT_KEYSTORE} component={ConfigureExitKeystore} />
        <Route path={paths.CREATE_EXIT} render={() => <div>create exit page</div>} />
      </ExitContext.Provider>
    </MemoryRouter>,
  );
  return spies;
}

async function pickKeystore() {
  openDialog.mockResolvedValue(KEYSTORE);
  keystorePubkey.mockResolvedValue("b3e4");
  fireEvent.click(screen.getByText("Select keystore file"));
  await screen.findByText("Validator public key: 0xb3e4");
}

function clickNext() {
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
}

afterEach(() => {
  vi.clearAllMocks();
});

describe("ConfigureExitKeystore", () => {
  it("requires a keystore file and its password", () => {
    const spies = renderPage();
    fireEvent.change(screen.getByLabelText(/^Validator index/), { target: { value: "7" } });
    clickNext();

    expect(screen.getByText(errors.KEYSTORE_REQUIRED)).toBeInTheDocument();
    expect(screen.getByText(errors.KEYSTORE_PASSWORD_REQUIRED)).toBeInTheDocument();
    expect(spies.setKeystorePath).not.toHaveBeenCalled();
    expect(screen.queryByText("create exit page")).not.toBeInTheDocument();
  });

  it("rejects a negative validator index", async () => {
    renderPage();
    await pickKeystore();
    fireEvent.change(screen.getByLabelText(/^Keystore password/), { target: { value: "MyPasswordIs" } });
    fireEvent.change(screen.getByLabelText(/^Validator index/), { target: { value: "-1" } });
    clickNext();

    expect(screen.getByText(errors.VALIDATOR_INDEX)).toBeInTheDocument();
  });

  it("stores the keystore, password, index and epoch and moves on", async () => {
    const spies = renderPage();
    await pickKeystore();
    fireEvent.change(screen.getByLabelText(/^Keystore password/), { target: { value: "MyPasswordIs" } });
    fireEvent.change(screen.getByLabelText(/^Validator index/), { target: { value: "7" } });
    fireEvent.change(screen.getByLabelText("Exit epoch"), { target: { value: "99" } });
    clickNext();

    expect(screen.getByText("create exit page")).toBeInTheDocument();
    expect(spies.setSource).toHaveBeenCalledWith("keystore");
    expect(spies.setKeystorePath).toHaveBeenCalledWith(KEYSTORE);
    expect(spies.setKeystorePassword).toHaveBeenCalledWith("MyPasswordIs");
    expect(spies.setValidatorIndex).toHaveBeenCalledWith(7);
    expect(spies.setEpoch).toHaveBeenCalledWith(99);
  });

  it("keeps a previously chosen keystore when coming back to the form", () => {
    renderPage({ keystorePath: KEYSTORE, keystorePassword: "MyPasswordIs", validatorIndex: 7 });
    expect(screen.getByText(`You've selected: ${KEYSTORE}`)).toBeInTheDocument();
    expect(screen.getByLabelText(/^Validator index/)).toHaveValue(7);
  });
});
