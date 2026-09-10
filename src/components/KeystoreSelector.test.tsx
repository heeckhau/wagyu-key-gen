import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { electronAPI, eth2Deposit } from "../api";
import KeystoreSelector from "./KeystoreSelector";

vi.mock("../api", () => ({
  electronAPI: { invokeShowOpenKeystoreDialog: vi.fn() },
  eth2Deposit: { keystorePubkey: vi.fn() },
}));

const openDialog = vi.mocked(electronAPI.invokeShowOpenKeystoreDialog);
const keystorePubkey = vi.mocked(eth2Deposit.keystorePubkey);

afterEach(() => {
  vi.clearAllMocks();
});

describe("KeystoreSelector", () => {
  it("shows the validator public key of a picked keystore and reports the selection", async () => {
    openDialog.mockResolvedValue("/keys/keystore-m_12381_3600_0_0_0-1.json");
    keystorePubkey.mockResolvedValue("b3e4");
    const onKeystoreSelect = vi.fn();
    render(<KeystoreSelector keystorePath="" onKeystoreSelect={onKeystoreSelect} />);

    fireEvent.click(screen.getByText("Select keystore file"));

    expect(await screen.findByText("Validator public key: 0xb3e4")).toBeInTheDocument();
    expect(keystorePubkey).toHaveBeenCalledWith("/keys/keystore-m_12381_3600_0_0_0-1.json");
    expect(onKeystoreSelect).toHaveBeenCalledWith("/keys/keystore-m_12381_3600_0_0_0-1.json", "b3e4");
  });

  it("shows the backend error for a file that is not a keystore", async () => {
    openDialog.mockResolvedValue("/keys/deposit_data-1.json");
    keystorePubkey.mockRejectedValue("/keys/deposit_data-1.json is not a valid keystore file.");
    const onKeystoreSelect = vi.fn();
    render(<KeystoreSelector keystorePath="" onKeystoreSelect={onKeystoreSelect} />);

    fireEvent.click(screen.getByText("Select keystore file"));

    expect(await screen.findByText(/is not a valid keystore file/)).toBeInTheDocument();
    expect(onKeystoreSelect).not.toHaveBeenCalled();
  });

  it("does nothing when the picker is cancelled and keeps showing the previous choice", async () => {
    openDialog.mockResolvedValue(null);
    const onKeystoreSelect = vi.fn();
    render(<KeystoreSelector keystorePath="/keys/previous.json" onKeystoreSelect={onKeystoreSelect} />);

    fireEvent.click(screen.getByText("Select keystore file"));

    await waitFor(() => expect(openDialog).toHaveBeenCalled());
    expect(screen.getByText("You've selected: /keys/previous.json")).toBeInTheDocument();
    expect(keystorePubkey).not.toHaveBeenCalled();
    expect(onKeystoreSelect).not.toHaveBeenCalled();
  });
});
