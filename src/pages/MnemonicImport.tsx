import { Button, TextField } from "@mui/material";
import { useContext, useState } from "react";
import { useHistory } from "react-router-dom";

import { eth2Deposit } from "../api";
import { BTECContext } from "../BTECContext";
import Loader from "../components/Loader";
import WizardWrapper from "../components/WizardWrapper";
import {
  BTECFlow,
  ExistingMnemonicFlow,
  ExitMnemonicFlow,
  MNEMONIC_ERROR_SEARCH,
  VALID_MNEMONIC_LENGTHS,
  errors,
  paths,
} from "../constants";
import { ExitContext } from "../ExitContext";
import { cleanMnemonic } from "../helpers";
import { KeyCreationContext } from "../KeyCreationContext";

/**
 * Allows the user to import an existing mnemonic to kickstart either the
 * validator key creation, the withdrawal credentials change or the exit flow
 */
const MnemonicImport = () => {
  const {mnemonic: btecMnemonic, setMnemonic: setBTECMnemonic} = useContext(BTECContext);
  const {mnemonic: exitMnemonic, setMnemonic: setExitMnemonic} = useContext(ExitContext);
  const {mnemonic, setMnemonic} = useContext(KeyCreationContext);
  const history = useHistory();
  const usingBTEC = history.location.pathname === paths.BTEC_IMPORT;
  const usingExit = history.location.pathname === paths.EXIT_IMPORT;

  const [error, setError] = useState("");
  const [inputMnemonic, setInputMnemonic] = useState(usingBTEC ? btecMnemonic : usingExit ? exitMnemonic : mnemonic);
  const [validatingMnemonic, setValidatingMnemonic] = useState(false);

  /**
   * Verifies the mnemonic is valid and will notify the user if not
   */
  const verifyMnemonic = () => {
    setError("");

    const cleanedMnemonic = cleanMnemonic(inputMnemonic);
    setInputMnemonic(cleanedMnemonic);

    const mnemonicArray = cleanedMnemonic.split(" ");

    if (!VALID_MNEMONIC_LENGTHS.includes(mnemonicArray.length)) {
      setError(errors.MNEMONIC_LENGTH_ERROR);
    } else {
      setValidatingMnemonic(true);
      eth2Deposit.validateMnemonic(cleanedMnemonic).then((validatedMnemonic) => {
        // The backend expands abbreviated words, so keep the full-word mnemonic.
        if (usingBTEC) {
          setBTECMnemonic(validatedMnemonic);
        } else if (usingExit) {
          setExitMnemonic(validatedMnemonic);
        } else {
          setMnemonic(validatedMnemonic);
        }
        setValidatingMnemonic(false);
        history.push(usingBTEC ? paths.CONFIGURE_BTEC : usingExit ? paths.CONFIGURE_EXIT : paths.CONFIGURE_EXISTING);
      }).catch((error) => {
        const errorMsg = String(error);

        if (errorMsg.indexOf(MNEMONIC_ERROR_SEARCH) >= 0) {
          setError(errors.INVALID_MNEMONIC_ERROR);
        } else {
          setError(errorMsg);
        }

        setValidatingMnemonic(false);
      });
    }
  };

  const handleKeyDown = (evt: React.KeyboardEvent<HTMLInputElement>) => {
    if (evt.key === 'Enter') {
      verifyMnemonic();
    }
  }

  const onBackClick = () => {
    history.replace("/");
  };

  const onNextClick = () => {
    verifyMnemonic();
  };

  return (
    <WizardWrapper
      actionBarItems={[
        <Button variant="contained" color="primary" disabled={validatingMnemonic} onClick={() => onBackClick()} tabIndex={3}>Back</Button>,
        <Button variant="contained" color="primary" disabled={validatingMnemonic || !inputMnemonic} onClick={() => onNextClick()} tabIndex={2}>Import</Button>,
      ]}
      activeTimelineIndex={0}
      timelineItems={usingBTEC ? BTECFlow : usingExit ? ExitMnemonicFlow : ExistingMnemonicFlow}
      title="Import Secret Recovery Phrase"
    >
      <div className="tw-px-20">
        { validatingMnemonic ? (
          <Loader message="Validating secret recovery phrase..." />
        ) : (
          <TextField
            className="tw-mr-2"
            id="mnemonic-input"
            label="Type your Secret Recovery Phrase here"
            multiline
            fullWidth
            rows={4}
            variant="outlined"
            color="primary"
            autoFocus
            error={!!error}
            helperText={error}
            value={inputMnemonic}
            onChange={(e) => setInputMnemonic(e.target.value)}
            onKeyDown={handleKeyDown}
          />
        )}
      </div>
    </WizardWrapper>
  );
};

export default MnemonicImport;
