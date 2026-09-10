import {
  Button,
  Checkbox,
  FormControlLabel,
  TextField,
  Tooltip,
  Typography,
} from "@mui/material";
import { useContext, useState } from "react";
import { useHistory } from "react-router-dom";

import { web3Utils } from "../api";
import KeystoreSelector from "../components/KeystoreSelector";
import WizardWrapper from "../components/WizardWrapper";
import {
  PartialDepositFlow,
  errors,
  formatCompoundingTooltip,
  formatDepositAmountError,
  formatTopUpAmountTooltip,
  getDepositAmountLimits,
  paths,
  tooltips,
} from "../constants";
import { GlobalContext } from "../GlobalContext";
import { trimAmountPrecision } from "../helpers";
import { PartialDepositContext } from "../PartialDepositContext";

/**
 * Form to pick a keystore file, its password, the amount to add and the validator's withdrawal
 * credentials, for a deposit that tops up an existing validator.
 */
const ConfigurePartialDeposit = () => {
  const { network } = useContext(GlobalContext);
  const {
    keystorePath,
    setKeystorePath,
    keystorePassword,
    setKeystorePassword,
    amount,
    setAmount,
    withdrawalAddress,
    setWithdrawalAddress,
    compounding,
    setCompounding,
  } = useContext(PartialDepositContext);
  const history = useHistory();

  const { min: minAmount, max: maxAmount } = getDepositAmountLimits(network);

  const [inputKeystorePath, setInputKeystorePath] = useState(keystorePath);
  const [keystoreError, setKeystoreError] = useState("");
  const [inputPassword, setInputPassword] = useState(keystorePassword);
  const [passwordError, setPasswordError] = useState("");
  const [inputAmount, setInputAmount] = useState(amount);
  const [amountError, setAmountError] = useState("");
  const [inputWithdrawalAddress, setInputWithdrawalAddress] = useState(withdrawalAddress);
  const [withdrawalAddressError, setWithdrawalAddressError] = useState("");
  const [inputCompounding, setInputCompounding] = useState(compounding);

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

    const numericAmount = Number(inputAmount);
    if (inputAmount.trim() === "" || Number.isNaN(numericAmount) || numericAmount < minAmount || numericAmount > maxAmount) {
      setAmountError(formatDepositAmountError(network));
      isError = true;
    } else {
      setAmountError("");
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

    if (isError) {
      return;
    }

    setKeystorePath(inputKeystorePath);
    setKeystorePassword(inputPassword);
    setAmount(inputAmount.trim());
    setWithdrawalAddress(inputWithdrawalAddress);
    setCompounding(inputCompounding);
    history.push(paths.CREATE_PARTIAL_DEPOSIT);
  };

  const onBackClick = () => {
    setKeystorePath("");
    setKeystorePassword("");
    setAmount("");
    setWithdrawalAddress("");
    setCompounding(false);
    history.goBack();
  };

  return (
    <WizardWrapper
      actionBarItems={[
        <Button variant="contained" color="primary" onClick={() => onBackClick()} tabIndex={3}>Back</Button>,
        <Button variant="contained" color="primary" onClick={() => validateInputs()} tabIndex={2}>Next</Button>,
      ]}
      activeTimelineIndex={0}
      timelineItems={PartialDepositFlow}
      title="Top up a validator"
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

          <Tooltip title={formatTopUpAmountTooltip(network)}>
            <TextField
              className="tw-w-[200px]"
              id="amount"
              label="Deposit Amount"
              type="number"
              variant="outlined"
              value={inputAmount}
              onChange={(e) => setInputAmount(trimAmountPrecision(e.target.value))}
              error={!!amountError}
              helperText={amountError}
              required
            />
          </Tooltip>
        </div>

        <div className="tw-flex tw-flex-row tw-gap-4 tw-items-center">
          <Tooltip title={tooltips.PARTIAL_DEPOSIT_WITHDRAW_ADDRESS}>
            <TextField
              className="tw-flex-grow"
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

          <Tooltip title={formatCompoundingTooltip(network)}>
            <FormControlLabel
              label="Compounding Credentials (0x02)"
              control={
                <Checkbox
                  checked={inputCompounding}
                  onChange={(e) => setInputCompounding(e.target.checked)}
                />
              }
            />
          </Tooltip>
        </div>

        <Typography className="tw-text-center" variant="body1">
          The withdrawal address and credential type must match the validator's current withdrawal credentials, otherwise the deposit will not be credited to it.
        </Typography>
      </div>
    </WizardWrapper>
  );
};

export default ConfigurePartialDeposit;
