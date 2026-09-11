import { Button, Typography } from "@mui/material";
import { useContext, useEffect, useState } from "react";
import { useHistory } from "react-router-dom";

import { eth2Deposit } from "../api";
import { BTECContext } from "../BTECContext";
import FolderSelector from "../components/FolderSelector";
import Loader from "../components/Loader";
import WizardWrapper from "../components/WizardWrapper";
import { BTECFlow, BTECKeystoreFlow, paths } from "../constants";
import { GlobalContext } from "../GlobalContext";

/**
 * Allows the user to select a folder where the credentials will be saved
 * and after which will attempt to generate the credential change and save
 * to the specified folder
 */
const CreateCredentialsChange = () => {
  const { network } = useContext(GlobalContext);
  const {
    source,
    btecCredentials,
    btecIndices,
    setFolderLocation,
    index,
    mnemonic,
    withdrawalAddress,
    keystorePath,
    keystorePassword,
    validatorIndex,
  } = useContext(BTECContext);
  const history = useHistory();
  const usingKeystore = source === "keystore";

  const [creatingCredentialsChange, setCreatingCredentialsChange] = useState(false);
  const [generationError, setGenerationError] = useState("");
  const [selectedFolder, setSelectedFolder] = useState("");

  useEffect(() => {
    if (usingKeystore ? !keystorePath : !mnemonic) {
      history.replace(usingKeystore ? paths.CONFIGURE_BTEC_KEYSTORE : paths.BTEC_IMPORT);
    }
  }, []);

  const onFolderSelect = (folder: string) => {
    setSelectedFolder(folder);
  };

  /**
   * Attempts to generate the credentials change and if successful send the user
   * to the final step of the flow
   */
  const handleBTECFileGeneration = () => {
    setCreatingCredentialsChange(true);
    let appendedWithdrawalAddress = withdrawalAddress;

    if (withdrawalAddress != "" && !withdrawalAddress.toLowerCase().startsWith("0x")) {
      appendedWithdrawalAddress = "0x" + withdrawalAddress;
    }

    const generation = usingKeystore
      ? eth2Deposit.generateBLSChangeKeystore(
          selectedFolder,
          network,
          keystorePath,
          keystorePassword,
          validatorIndex,
          appendedWithdrawalAddress,
        )
      : eth2Deposit.generateBLSChange(
          selectedFolder,
          network,
          mnemonic,
          index,
          btecIndices,
          btecCredentials,
          appendedWithdrawalAddress,
        );

    generation.then(() => {
      setFolderLocation(selectedFolder);
      history.push(paths.FINISH_CREDENTIALS);
    }).catch((error) => {
      const errorMsg = String(error);
      setGenerationError(errorMsg);
      setCreatingCredentialsChange(false);
    });
  }

  const onBackClick = () => {
    setSelectedFolder("");
    history.goBack();
  };

  const onNextClick = () => {
    if (selectedFolder) {
      handleBTECFileGeneration();
    }
  };

  return (
    <WizardWrapper
      actionBarItems={creatingCredentialsChange ? [] : [
        <Button variant="contained" color="primary" onClick={() => onBackClick()} tabIndex={3}>Back</Button>,
        <Button variant="contained" color="primary" disabled={!selectedFolder} onClick={() => onNextClick()} tabIndex={2}>Create</Button>,
      ]}
      activeTimelineIndex={usingKeystore ? 1 : 2}
      timelineItems={usingKeystore ? BTECKeystoreFlow : BTECFlow}
      title="Generate BLS to execution change"
    >
      { creatingCredentialsChange ? (
        <Loader message="Creating your BLS to execution change file." />
      ) : (
        <div className="tw-flex tw-flex-col tw-items-center tw-gap-4">
          <Typography>Choose a folder where we should save your BLS to execution change file.</Typography>

          <FolderSelector onFolderSelect={onFolderSelect} />

          {selectedFolder && <Typography>You've selected: {selectedFolder}</Typography>}

          {generationError && <Typography color="error">{generationError}</Typography>}
        </div>
      )}
    </WizardWrapper>
  )
};

export default CreateCredentialsChange;
