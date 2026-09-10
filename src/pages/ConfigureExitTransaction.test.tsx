import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route } from "react-router-dom";
import { describe, expect, it } from "vitest";

import { errors, paths } from "../constants";
import { ExitContext } from "../ExitContext";
import { exitContextValue } from "../testing/exitContext";
import ConfigureExitTransaction from "./ConfigureExitTransaction";

const MNEMONIC =
  "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

function renderPage(overrides: Record<string, unknown> = {}) {
  const { value, spies } = exitContextValue({ mnemonic: MNEMONIC, ...overrides });
  render(
    <MemoryRouter initialEntries={[paths.CONFIGURE_EXIT]}>
      <ExitContext.Provider value={value}>
        <Route path={paths.CONFIGURE_EXIT} component={ConfigureExitTransaction} />
        <Route path={paths.CREATE_EXIT} render={() => <div>create exit page</div>} />
        <Route path={paths.EXIT_IMPORT} render={() => <div>import page</div>} />
      </ExitContext.Provider>
    </MemoryRouter>,
  );
  return spies;
}

function fillForm({ index = "0", indices = "1,2", epoch = "0" } = {}) {
  fireEvent.change(screen.getByLabelText(/^Start index/), { target: { value: index } });
  fireEvent.change(screen.getByLabelText(/^Validator indexes/), { target: { value: indices } });
  fireEvent.change(screen.getByLabelText("Exit epoch"), { target: { value: epoch } });
}

function clickNext() {
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
}

describe("ConfigureExitTransaction", () => {
  it("rejects indices that aren't plain digits", () => {
    const spies = renderPage();
    fillForm({ indices: "1,x" });
    clickNext();

    expect(screen.getByText(errors.INDICES_FORMAT)).toBeInTheDocument();
    expect(spies.setIndices).not.toHaveBeenCalled();
    expect(screen.queryByText("create exit page")).not.toBeInTheDocument();
  });

  it("requires at least one validator index", () => {
    renderPage();
    fillForm({ indices: "" });
    clickNext();

    expect(screen.getByText(errors.INDICES)).toBeInTheDocument();
  });

  it("rejects a negative start index and a non-numeric epoch", () => {
    renderPage();
    fillForm({ index: "-1", epoch: "abc" });
    clickNext();

    expect(screen.getByText(errors.NON_NEGATIVE_INDEX)).toBeInTheDocument();
    expect(screen.getByText(errors.EPOCH)).toBeInTheDocument();
  });

  it("stores the trimmed inputs in the context and moves on", () => {
    const spies = renderPage();
    fillForm({ index: "3", indices: " 10, 11 ", epoch: "1234" });
    clickNext();

    expect(screen.getByText("create exit page")).toBeInTheDocument();
    expect(spies.setSource).toHaveBeenCalledWith("mnemonic");
    expect(spies.setIndex).toHaveBeenCalledWith(3);
    expect(spies.setIndices).toHaveBeenCalledWith("10, 11");
    expect(spies.setEpoch).toHaveBeenCalledWith(1234);
  });

  it("goes back to the import step when there is no mnemonic", () => {
    renderPage({ mnemonic: "" });
    expect(screen.getByText("import page")).toBeInTheDocument();
  });
});
