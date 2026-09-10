import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route } from "react-router-dom";
import { describe, expect, it } from "vitest";

import { paths } from "../constants";
import GlobalContextWrapper from "../GlobalContext";
import Home from "./Home";

function renderHome() {
  render(
    <MemoryRouter initialEntries={["/"]}>
      <GlobalContextWrapper>
        <Route exact path="/" component={Home} />
        <Route path={paths.CREATE_MNEMONIC} render={() => <div>create mnemonic page</div>} />
        <Route path={paths.EXISTING_IMPORT} render={() => <div>import existing page</div>} />
        <Route path={paths.BTEC_IMPORT} render={() => <div>import btec page</div>} />
        <Route path={paths.EXIT_IMPORT} render={() => <div>import exit page</div>} />
        <Route path={paths.CONFIGURE_EXIT_KEYSTORE} render={() => <div>configure exit keystore page</div>} />
        <Route path={paths.CONFIGURE_PARTIAL_DEPOSIT} render={() => <div>configure partial deposit page</div>} />
        <Route path={paths.CONFIGURE_BTEC_KEYSTORE} render={() => <div>configure btec keystore page</div>} />
      </GlobalContextWrapper>
    </MemoryRouter>,
  );
}

// Buttons wrapped in a Tooltip get the tooltip as their accessible name, so they are found by
// their visible text rather than by role.
function click(text: string | RegExp) {
  fireEvent.click(screen.getByText(text));
}

/** Every entry point first asks for the network; confirming it continues the chosen action. */
function confirmNetwork() {
  fireEvent.click(screen.getByRole("button", { name: "OK" }));
}

const BLS_CHANGE = /^Generate your BLS to execution change/;

describe("Home", () => {
  it("asks for the network first, then starts the mnemonic creation", () => {
    renderHome();
    click("Create New Secret Recovery Phrase");
    expect(screen.queryByText("create mnemonic page")).not.toBeInTheDocument();

    confirmNetwork();
    expect(screen.getByText("create mnemonic page")).toBeInTheDocument();
  });

  it.each([
    ["Generate existing or new validator keys", "import existing page"],
    [BLS_CHANGE, "import btec page"],
    ["Exit your validator(s)", "import exit page"],
  ])("routes the existing-mnemonic action %s", (action, page) => {
    renderHome();
    click("Use Existing Secret Recovery Phrase");
    confirmNetwork();

    click(action);
    expect(screen.getByText(page)).toBeInTheDocument();
  });

  it.each([
    ["Exit a validator", "configure exit keystore page"],
    ["Top up a validator", "configure partial deposit page"],
    [BLS_CHANGE, "configure btec keystore page"],
  ])("routes the keystore action %s", (action, page) => {
    renderHome();
    click("Use Existing Keystore File");
    confirmNetwork();

    click(action);
    expect(screen.getByText(page)).toBeInTheDocument();
  });

  it("only asks for the network once", () => {
    renderHome();
    click("Use Existing Keystore File");
    confirmNetwork();
    fireEvent.keyDown(screen.getByText("Top up a validator"), { key: "Escape" });

    click("Use Existing Secret Recovery Phrase");
    expect(screen.queryByRole("button", { name: "OK" })).not.toBeInTheDocument();
    expect(screen.getByText("Exit your validator(s)")).toBeInTheDocument();
  });
});
