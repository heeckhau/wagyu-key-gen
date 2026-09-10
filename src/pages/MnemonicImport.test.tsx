import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";

import { eth2Deposit } from "../api";
import BTECContextWrapper from "../BTECContext";
import { errors, paths } from "../constants";
import KeyCreationContextWrapper from "../KeyCreationContext";
import MnemonicImport from "./MnemonicImport";

vi.mock("../api", () => ({
  eth2Deposit: { validateMnemonic: vi.fn() },
}));

const validateMnemonic = vi.mocked(eth2Deposit.validateMnemonic);

const ABANDON = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

function renderPage() {
  return render(
    <MemoryRouter initialEntries={[paths.EXISTING_IMPORT]}>
      <KeyCreationContextWrapper>
        <BTECContextWrapper>
          <Route path={paths.EXISTING_IMPORT} component={MnemonicImport} />
          <Route path={paths.CONFIGURE_EXISTING} render={() => <div>configure page</div>} />
        </BTECContextWrapper>
      </KeyCreationContextWrapper>
    </MemoryRouter>,
  );
}

function typeMnemonic(value: string) {
  fireEvent.change(screen.getByLabelText("Type your Secret Recovery Phrase here"), { target: { value } });
  fireEvent.click(screen.getByRole("button", { name: "Import" }));
}

afterEach(() => {
  vi.clearAllMocks();
});

describe("MnemonicImport", () => {
  it("rejects a wrong word count before asking the backend", () => {
    renderPage();
    typeMnemonic(`${ABANDON} extra`);
    expect(screen.getByText(errors.MNEMONIC_LENGTH_ERROR)).toBeInTheDocument();
    expect(validateMnemonic).not.toHaveBeenCalled();
  });

  it("cleans the input, validates it and moves on with the full-word mnemonic", async () => {
    validateMnemonic.mockResolvedValue(ABANDON);
    renderPage();
    typeMnemonic("aban, aban aban  aban aban aban aban aban aban aban aban abou ");
    expect(validateMnemonic).toHaveBeenCalledWith("aban aban aban aban aban aban aban aban aban aban aban abou");
    expect(await screen.findByText("configure page")).toBeInTheDocument();
  });

  it("shows the friendly error for an invalid mnemonic", async () => {
    validateMnemonic.mockRejectedValue("That is not a valid mnemonic, please check for typos.");
    renderPage();
    typeMnemonic(ABANDON);
    expect(await screen.findByText(errors.INVALID_MNEMONIC_ERROR)).toBeInTheDocument();
  });

  it("shows other backend errors verbatim", async () => {
    validateMnemonic.mockRejectedValue("Multiple valid languages found: chinese_simplified, chinese_traditional. Please specify the mnemonic language.");
    renderPage();
    typeMnemonic("的 的 的 的 的 的 的 的 的 的 的 在");
    expect(await screen.findByText(/Multiple valid languages found/)).toBeInTheDocument();
  });
});
