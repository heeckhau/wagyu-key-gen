import { Button, Typography } from "@mui/material";
import { useContext, useEffect, useState } from "react";
import { useHistory } from "react-router-dom";

import { eth2Deposit } from "../api";
import FolderSelector from "../components/FolderSelector";
import Loader from "../components/Loader";
import WizardWrapper from "../components/WizardWrapper";
import { ExitKeystoreFlow, ExitMnemonicFlow, paths } from "../constants";
import { ExitContext } from "../ExitContext";
import { GlobalContext } from "../GlobalContext";

/**
 * Allows the user to select a folder where the signed exit(s) will be saved and then
 * generates them, either from the Secret Recovery Phrase or from the chosen keystore.
 */
const CreateExitTransaction = () => {
  const { network } = useContext(GlobalContext);
  const {
    source,
    mnemonic,
    index,
    indices,
    keystorePath,
    keystorePassword,
    validatorIndex,
    epoch,
    setFolderLocation,
  } = useContext(ExitContext);
  const history = useHistory();
  const usingKeystore = source === "keystore";

  const [creatingExit, setCreatingExit] = useState(false);
  const [generationError, setGenerationError] = useState("");
  const [selectedFolder, setSelectedFolder] = useState("");

  useEffect(() => {
    if (usingKeystore ? !keystorePath : !mnemonic) {
      history.replace(usingKeystore ? paths.CONFIGURE_EXIT_KEYSTORE : paths.EXIT_IMPORT);
    }
  }, []);

  const createExit = () => {
    setCreatingExit(true);

    const generation = usingKeystore
      ? eth2Deposit.generateExitTransactionKeystore(selectedFolder, network, keystorePath, keystorePassword, validatorIndex, epoch)
      : eth2Deposit.generateExitTransactions(selectedFolder, network, mnemonic, index, indices, epoch);

    generation.then(() => {
      setFolderLocation(selectedFolder);
      history.push(paths.FINISH_EXIT);
    }).catch((error) => {
      setGenerationError(String(error));
      setCreatingExit(false);
    });
  };

  const onBackClick = () => {
    setSelectedFolder("");
    history.goBack();
  };

  const onNextClick = () => {
    if (selectedFolder) {
      createExit();
    }
  };

  return (
    <WizardWrapper
      actionBarItems={creatingExit ? [] : [
        <Button variant="contained" color="primary" onClick={() => onBackClick()} tabIndex={3}>Back</Button>,
        <Button variant="contained" color="primary" disabled={!selectedFolder} onClick={() => onNextClick()} tabIndex={2}>Create</Button>,
      ]}
      activeTimelineIndex={usingKeystore ? 1 : 2}
      timelineItems={usingKeystore ? ExitKeystoreFlow : ExitMnemonicFlow}
      title="Generate exit transaction"
    >
      { creatingExit ? (
        <Loader message="Creating your signed exit transaction file(s)." />
      ) : (
        <div className="tw-flex tw-flex-col tw-items-center tw-gap-4">
          <Typography>Choose a folder where we should save your signed exit transaction file(s).</Typography>

          <FolderSelector onFolderSelect={setSelectedFolder} />

          {selectedFolder && <Typography>You've selected: {selectedFolder}</Typography>}

          {generationError && <Typography color="error">{generationError}</Typography>}
        </div>
      )}
    </WizardWrapper>
  );
};

export default CreateExitTransaction;
