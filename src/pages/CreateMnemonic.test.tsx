import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";

import { eth2Deposit } from "../api";
import { GlobalContext } from "../GlobalContext";
import KeyCreationContextWrapper, { KeyCreationContext } from "../KeyCreationContext";
import { paths } from "../constants";
import { Network } from "../types";
import CreateMnemonic from "./CreateMnemonic";

vi.mock("../api", () => ({
  eth2Deposit: { createMnemonic: vi.fn() },
  electronAPI: { clipboardWriteText: vi.fn() },
}));

const createMnemonic = vi.mocked(eth2Deposit.createMnemonic);

const GENERATED =
  "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

// HOODI renders the phrase confirmation as a single textarea instead of a 24-box grid,
// which is exercised on its own in VerifyMnemonic.test.tsx.
function renderPage() {
  return render(
    <MemoryRouter initialEntries={[paths.CREATE_MNEMONIC]}>
      <GlobalContext.Provider value={{ network: Network.HOODI, setNetwork: () => {} }}>
        <KeyCreationContextWrapper>
          <Route path={paths.CREATE_MNEMONIC} component={CreateMnemonic} />
          <Route path={paths.CONFIGURE_CREATE} render={() => <div>configure page</div>} />
        </KeyCreationContextWrapper>
      </GlobalContext.Provider>
    </MemoryRouter>,
  );
}

async function generateAndReachConfirmStep() {
  createMnemonic.mockResolvedValue(GENERATED);
  renderPage();

  fireEvent.click(screen.getByRole("button", { name: "Create" }));
  await screen.findByRole("button", { name: "Next" });
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
  fireEvent.click(await screen.findByRole("button", { name: "I'm Sure" }));
  await screen.findByRole("button", { name: "Check" });
}

afterEach(() => {
  vi.clearAllMocks();
});

describe("CreateMnemonic", () => {
  it("accepts a retyped phrase that only differs by punctuation and whitespace", async () => {
    await generateAndReachConfirmStep();

    fireEvent.change(screen.getByLabelText("Confirm your Secret Recovery Phrase"), {
      target: {
        value:
          "abandon, abandon  abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
      },
    });
    fireEvent.click(screen.getByRole("button", { name: "Check" }));

    expect(await screen.findByText("configure page")).toBeInTheDocument();
  });

  it("flags a retyped phrase that actually differs", async () => {
    await generateAndReachConfirmStep();

    fireEvent.change(screen.getByLabelText("Confirm your Secret Recovery Phrase"), {
      target: {
        value: "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon zebra",
      },
    });
    fireEvent.click(screen.getByRole("button", { name: "Check" }));

    expect(await screen.findByText(/does not match what was given to you/)).toBeInTheDocument();
    expect(screen.queryByText("configure page")).not.toBeInTheDocument();
  });

  it("resumes straight to the confirmation step when the mnemonic was already generated", () => {
    render(
      <MemoryRouter initialEntries={[paths.CREATE_MNEMONIC]}>
        <GlobalContext.Provider value={{ network: Network.HOODI, setNetwork: () => {} }}>
          <KeyCreationContext.Provider
            value={{
              folderLocation: "",
              setFolderLocation: () => {},
              index: 0,
              setIndex: () => {},
              mnemonic: GENERATED,
              setMnemonic: () => {},
              numberOfKeys: 1,
              setNumberOfKeys: () => {},
              amount: 32,
              setAmount: () => {},
              password: "",
              setPassword: () => {},
              withdrawalAddress: "",
              setWithdrawalAddress: () => {},
              compounding: false,
              setCompounding: () => {},
            }}
          >
            <Route path={paths.CREATE_MNEMONIC} component={CreateMnemonic} />
          </KeyCreationContext.Provider>
        </GlobalContext.Provider>
      </MemoryRouter>,
    );

    expect(screen.getByLabelText("Confirm your Secret Recovery Phrase")).toHaveValue(GENERATED);
    expect(screen.getByRole("button", { name: "Check" })).toBeInTheDocument();
  });
});
