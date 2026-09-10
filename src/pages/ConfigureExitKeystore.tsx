import { Button, TextField, Tooltip, Typography } from "@mui/material";
import { useContext, useState } from "react";
import { useHistory } from "react-router-dom";

import KeystoreSelector from "../components/KeystoreSelector";
import WizardWrapper from "../components/WizardWrapper";
import { ExitKeystoreFlow, errors, paths, tooltips } from "../constants";
import { ExitContext } from "../ExitContext";
import { parseNonNegativeInt } from "../helpers";

/**
 * Form to pick a keystore file, its password, the validator index and the exit epoch, for an
 * exit signed with the key in the keystore.
 */
const ConfigureExitKeystore = () => {
  const {
    keystorePath,
    setKeystorePath,
    keystorePassword,
    setKeystorePassword,
    validatorIndex,
    setValidatorIndex,
    epoch,
    setEpoch,
    setSource,
  } = useContext(ExitContext);
  const history = useHistory();

  const [inputKeystorePath, setInputKeystorePath] = useState(keystorePath);
  const [keystoreError, setKeystoreError] = useState("");
  const [inputPassword, setInputPassword] = useState(keystorePassword);
  const [passwordError, setPasswordError] = useState("");
  const [inputValidatorIndex, setInputValidatorIndex] = useState(String(validatorIndex));
  const [validatorIndexError, setValidatorIndexError] = useState("");
  const [inputEpoch, setInputEpoch] = useState(String(epoch));
  const [epochError, setEpochError] = useState("");

  const validateInputs = () => {
    let isError = false;

    if (!inputKeystorePath) {
      setKeystoreError(errors.KEYSTORE_REQUIRED);
      isError = true;
    } else {
      setKeystoreError("");
    }

    if (!inputPassword) {
      setPasswordError(errors.KEYSTORE_PASSWORD_REQUIRED);
      isError = true;
    } else {
      setPasswordError("");
    }

    const parsedValidatorIndex = parseNonNegativeInt(inputValidatorIndex);
    if (parsedValidatorIndex === null) {
      setValidatorIndexError(errors.VALIDATOR_INDEX);
      isError = true;
    } else {
      setValidatorIndexError("");
    }

    const parsedEpoch = parseNonNegativeInt(inputEpoch);
    if (parsedEpoch === null) {
      setEpochError(errors.EPOCH);
      isError = true;
    } else {
      setEpochError("");
    }

    if (isError || parsedValidatorIndex === null || parsedEpoch === null) {
      return;
    }

    setSource("keystore");
    setKeystorePath(inputKeystorePath);
    setKeystorePassword(inputPassword);
    setValidatorIndex(parsedValidatorIndex);
    setEpoch(parsedEpoch);
    history.push(paths.CREATE_EXIT);
  };

  const onBackClick = () => {
    setKeystorePath("");
    setKeystorePassword("");
    setValidatorIndex(0);
    setEpoch(0);
    history.goBack();
  };

  return (
    <WizardWrapper
      actionBarItems={[
        <Button variant="contained" color="primary" onClick={() => onBackClick()} tabIndex={3}>Back</Button>,
        <Button variant="contained" color="primary" onClick={() => validateInputs()} tabIndex={2}>Next</Button>,
      ]}
      activeTimelineIndex={0}
      timelineItems={ExitKeystoreFlow}
      title="Generate exit transaction"
    >
      <div className="tw-flex tw-flex-col tw-gap-4 tw-mx-16">
        <KeystoreSelector keystorePath={inputKeystorePath} onKeystoreSelect={(path) => setInputKeystorePath(path)} />
        {keystoreError && <Typography className="tw-text-center" color="error">{keystoreError}</Typography>}

        <div className="tw-flex tw-flex-row tw-gap-4">
          <Tooltip title={tooltips.KEYSTORE_PASSWORD}>
            <TextField
              className="tw-flex-grow"
              id="keystore-password"
              label="Keystore password"
              type="password"
              variant="outlined"
              value={inputPassword}
              onChange={(e) => setInputPassword(e.target.value)}
              error={!!passwordError}
              helperText={passwordError}
              required
            />
          </Tooltip>

          <Tooltip title={tooltips.EXIT_VALIDATOR_INDEX}>
            <TextField
              className="tw-w-[200px]"
              id="validator-index"
              label="Validator index"
              variant="outlined"
              type="number"
              value={inputValidatorIndex}
              onChange={(e) => setInputValidatorIndex(e.target.value)}
              InputProps={{ inputProps: { min: 0 } }}
              error={!!validatorIndexError}
              helperText={validatorIndexError}
              required
            />
          </Tooltip>

          <Tooltip title={tooltips.EXIT_EPOCH}>
            <TextField
              className="tw-w-[200px]"
              id="epoch"
              label="Exit epoch"
              variant="outlined"
              type="number"
              value={inputEpoch}
              onChange={(e) => setInputEpoch(e.target.value)}
              InputProps={{ inputProps: { min: 0 } }}
              error={!!epochError}
              helperText={epochError}
            />
          </Tooltip>
        </div>

        <Typography className="tw-text-center" variant="body1">
          Exiting a validator is permanent. Once the signed exit is broadcast, the validator stops attesting and its balance is withdrawn to its withdrawal address.
        </Typography>
      </div>
    </WizardWrapper>
  );
};

export default ConfigureExitKeystore;
