import { Button, Tooltip, Typography } from "@mui/material";
import { useState } from "react";

import { electronAPI, eth2Deposit } from "../api";
import { tooltips } from "../constants";

interface KeystoreSelectorParams {
  /** The keystore currently selected, shown so the choice survives going back a step. */
  keystorePath: string;
  onKeystoreSelect: (path: string, pubkey: string) => void;
}

/**
 * Lets the user pick a keystore file. The file is checked by the backend before it is accepted,
 * and the validator public key it holds is shown so the user can confirm it is the right one.
 */
const KeystoreSelector = ({ keystorePath, onKeystoreSelect }: KeystoreSelectorParams) => {
  const [displayFilePicker, setDisplayFilePicker] = useState(false);
  const [pubkey, setPubkey] = useState("");
  const [errorMessage, setErrorMessage] = useState("");

  const verifyKeystore = (path: string) => {
    eth2Deposit.keystorePubkey(path)
      .then((key) => {
        setPubkey(key);
        onKeystoreSelect(path, key);
      })
      .catch((error) => {
        setPubkey("");
        setErrorMessage(String(error));
      });
  };

  const chooseKeystore = () => {
    setErrorMessage("");
    setDisplayFilePicker(true);

    electronAPI.invokeShowOpenKeystoreDialog()
      .then((path: string | null) => {
        if (path) {
          verifyKeystore(path);
        }
      })
      .finally(() => {
        setDisplayFilePicker(false);
      });
  };

  return (
    <div className="tw-flex tw-flex-col tw-items-center tw-gap-2">
      <Tooltip title={tooltips.KEYSTORE}>
        <Button
          color="secondary"
          component="label"
          disabled={displayFilePicker}
          onClick={chooseKeystore}
          tabIndex={1}
          variant="contained"
        >
          Select keystore file
        </Button>
      </Tooltip>
      {keystorePath && (
        <Typography variant="body2" className="tw-break-all">
          You've selected: {keystorePath}
        </Typography>
      )}
      {pubkey && (
        <Typography variant="body2" className="tw-text-cyan tw-break-all">
          Validator public key: 0x{pubkey}
        </Typography>
      )}
      {errorMessage && <Typography color="error">{errorMessage}</Typography>}
    </div>
  );
};

export default KeystoreSelector;
