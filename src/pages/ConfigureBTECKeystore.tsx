import { Button, TextField, Tooltip, Typography } from "@mui/material";
import { useContext, useState } from "react";
import { useHistory } from "react-router-dom";

import { web3Utils } from "../api";
import { BTECContext } from "../BTECContext";
import KeystoreSelector from "../components/KeystoreSelector";
import WizardWrapper from "../components/WizardWrapper";
import { BTECKeystoreFlow, errors, paths, tooltips } from "../constants";
import { parseNonNegativeInt } from "../helpers";

/**
 * Form to pick a keystore file, its password, the validator index and the withdrawal address to
 * set, for a BLS to execution change signed with the key in the keystore.
 */
const ConfigureBTECKeystore = () => {
  const {
    keystorePath,
    setKeystorePath,
    keystorePassword,
    setKeystorePassword,
    validatorIndex,
    setValidatorIndex,
    withdrawalAddress,
    setWithdrawalAddress,
    setSource,
  } = useContext(BTECContext);
  const history = useHistory();

  const [inputKeystorePath, setInputKeystorePath] = useState(keystorePath);
  const [keystoreError, setKeystoreError] = useState("");
  const [inputPassword, setInputPassword] = useState(keystorePassword);
  const [passwordError, setPasswordError] = useState("");
  const [inputValidatorIndex, setInputValidatorIndex] = useState(String(validatorIndex));
  const [validatorIndexError, setValidatorIndexError] = useState("");
  const [inputWithdrawalAddress, setInputWithdrawalAddress] = useState(withdrawalAddress);
  const [withdrawalAddressError, setWithdrawalAddressError] = useState("");

  const validateInputs = async () => {
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

    if (inputWithdrawalAddress === "") {
      setWithdrawalAddressError(errors.WITHDRAW_ADDRESS_REQUIRED);
      isError = true;
    } else if (!(await web3Utils.isAddress(inputWithdrawalAddress))) {
      setWithdrawalAddressError(errors.ADDRESS_FORMAT_ERROR);
      isError = true;
    } else {
      setWithdrawalAddressError("");
    }

    if (isError || parsedValidatorIndex === null) {
      return;
    }

    setSource("keystore");
    setKeystorePath(inputKeystorePath);
    setKeystorePassword(inputPassword);
    setValidatorIndex(parsedValidatorIndex);
    setWithdrawalAddress(inputWithdrawalAddress);
    history.push(paths.CREATE_CREDENTIALS);
  };

  const onBackClick = () => {
    setKeystorePath("");
    setKeystorePassword("");
    setValidatorIndex(0);
    setWithdrawalAddress("");
    history.goBack();
  };

  return (
    <WizardWrapper
      actionBarItems={[
        <Button variant="contained" color="primary" onClick={() => onBackClick()} tabIndex={3}>Back</Button>,
        <Button variant="contained" color="primary" onClick={() => validateInputs()} tabIndex={2}>Next</Button>,
      ]}
      activeTimelineIndex={0}
      timelineItems={BTECKeystoreFlow}
      title="Generate BLS to execution change"
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

          <Tooltip title={tooltips.BTEC_KEYSTORE_VALIDATOR_INDEX}>
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
        </div>

        <div className="tw-text-center">
          <Tooltip title={tooltips.BTEC_WITHDRAW_ADDRESS}>
            <TextField
              className="tw-w-[440px]"
              id="eth1-withdraw-address"
              label="Ethereum Withdrawal Address"
              variant="outlined"
              value={inputWithdrawalAddress}
              onChange={(e) => setInputWithdrawalAddress(e.target.value.trim())}
              error={!!withdrawalAddressError}
              helperText={withdrawalAddressError}
              required
            />
          </Tooltip>
          <Typography className="tw-mt-2" variant="body1">
            Please ensure that you have control over this address.
          </Typography>
        </div>
      </div>
    </WizardWrapper>
  );
};

export default ConfigureBTECKeystore;
