import { Button, TextField, Tooltip, Typography } from "@mui/material";
import { useContext, useEffect, useState } from "react";
import { useHistory } from "react-router-dom";

import WizardWrapper from "../components/WizardWrapper";
import { ExitMnemonicFlow, errors, paths, tooltips } from "../constants";
import { ExitContext } from "../ExitContext";
import { parseNonNegativeInt } from "../helpers";

/**
 * Form to provide the start index, the validator indices to exit and the exit epoch, for exits
 * signed with keys derived from the Secret Recovery Phrase.
 */
const ConfigureExitTransaction = () => {
  const {
    mnemonic,
    index,
    setIndex,
    indices,
    setIndices,
    epoch,
    setEpoch,
    setSource,
  } = useContext(ExitContext);
  const history = useHistory();

  const [inputIndex, setInputIndex] = useState(String(index));
  const [indexError, setIndexError] = useState("");
  const [inputIndices, setInputIndices] = useState(indices);
  const [indicesError, setIndicesError] = useState("");
  const [inputEpoch, setInputEpoch] = useState(String(epoch));
  const [epochError, setEpochError] = useState("");

  useEffect(() => {
    if (!mnemonic) {
      history.replace(paths.EXIT_IMPORT);
    }
  }, []);

  const validateInputs = () => {
    let isError = false;

    const parsedIndex = parseNonNegativeInt(inputIndex);
    if (parsedIndex === null) {
      setIndexError(errors.NON_NEGATIVE_INDEX);
      isError = true;
    } else {
      setIndexError("");
    }

    const trimmedIndices = inputIndices.trim();
    if (trimmedIndices === "") {
      setIndicesError(errors.INDICES);
      isError = true;
    } else if (!trimmedIndices.split(",").every((item) => /^\s*\d+\s*$/.test(item))) {
      setIndicesError(errors.INDICES_FORMAT);
      isError = true;
    } else {
      setIndicesError("");
    }

    const parsedEpoch = parseNonNegativeInt(inputEpoch);
    if (parsedEpoch === null) {
      setEpochError(errors.EPOCH);
      isError = true;
    } else {
      setEpochError("");
    }

    if (isError || parsedIndex === null || parsedEpoch === null) {
      return;
    }

    setSource("mnemonic");
    setIndex(parsedIndex);
    setIndices(trimmedIndices);
    setEpoch(parsedEpoch);
    history.push(paths.CREATE_EXIT);
  };

  const onBackClick = () => {
    setIndex(0);
    setIndices("");
    setEpoch(0);
    history.goBack();
  };

  return (
    <WizardWrapper
      actionBarItems={[
        <Button variant="contained" color="primary" onClick={() => onBackClick()} tabIndex={3}>Back</Button>,
        <Button variant="contained" color="primary" onClick={() => validateInputs()} tabIndex={2}>Next</Button>,
      ]}
      activeTimelineIndex={1}
      timelineItems={ExitMnemonicFlow}
      title="Generate exit transaction"
    >
      <div className="tw-flex tw-flex-col tw-gap-4 tw-mx-16">
        <div className="tw-flex tw-flex-row tw-gap-4">
          <Tooltip title={tooltips.EXIT_START_INDEX}>
            <TextField
              className="tw-w-[200px]"
              id="index"
              label="Start index"
              variant="outlined"
              type="number"
              value={inputIndex}
              onChange={(e) => setInputIndex(e.target.value)}
              InputProps={{ inputProps: { min: 0 } }}
              error={!!indexError}
              helperText={indexError}
              required
            />
          </Tooltip>

          <Tooltip title={tooltips.EXIT_INDICES}>
            <TextField
              className="tw-flex-grow"
              id="indices"
              label="Validator indexes (comma separated)"
              variant="outlined"
              value={inputIndices}
              onChange={(e) => setInputIndices(e.target.value)}
              error={!!indicesError}
              helperText={indicesError}
              required
            />
          </Tooltip>
        </div>

        <div className="tw-text-center">
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

export default ConfigureExitTransaction;
