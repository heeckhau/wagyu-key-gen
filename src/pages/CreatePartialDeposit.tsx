import { Button, Typography } from "@mui/material";
import { useContext, useEffect, useState } from "react";
import { useHistory } from "react-router-dom";

import { eth2Deposit } from "../api";
import FolderSelector from "../components/FolderSelector";
import Loader from "../components/Loader";
import WizardWrapper from "../components/WizardWrapper";
import { PartialDepositFlow, paths } from "../constants";
import { GlobalContext } from "../GlobalContext";
import { PartialDepositContext } from "../PartialDepositContext";

/**
 * Allows the user to select a folder where the deposit data will be saved and then generates it.
 */
const CreatePartialDeposit = () => {
  const { network } = useContext(GlobalContext);
  const {
    keystorePath,
    keystorePassword,
    amount,
    withdrawalAddress,
    compounding,
    setFolderLocation,
  } = useContext(PartialDepositContext);
  const history = useHistory();

  const [creatingDeposit, setCreatingDeposit] = useState(false);
  const [generationError, setGenerationError] = useState("");
  const [selectedFolder, setSelectedFolder] = useState("");

  useEffect(() => {
    if (!keystorePath) {
      history.replace(paths.CONFIGURE_PARTIAL_DEPOSIT);
    }
  }, []);

  const createDeposit = () => {
    setCreatingDeposit(true);

    let appendedWithdrawalAddress = withdrawalAddress;
    if (!withdrawalAddress.toLowerCase().startsWith("0x")) {
      appendedWithdrawalAddress = "0x" + withdrawalAddress;
    }

    eth2Deposit.generatePartialDeposit(
      selectedFolder,
      network,
      keystorePath,
      keystorePassword,
      amount,
      appendedWithdrawalAddress,
      compounding,
    ).then(() => {
      setFolderLocation(selectedFolder);
      history.push(paths.FINISH_PARTIAL_DEPOSIT);
    }).catch((error) => {
      setGenerationError(String(error));
      setCreatingDeposit(false);
    });
  };

  const onBackClick = () => {
    setSelectedFolder("");
    history.goBack();
  };

  const onNextClick = () => {
    if (selectedFolder) {
      createDeposit();
    }
  };

  return (
    <WizardWrapper
      actionBarItems={creatingDeposit ? [] : [
        <Button variant="contained" color="primary" onClick={() => onBackClick()} tabIndex={3}>Back</Button>,
        <Button variant="contained" color="primary" disabled={!selectedFolder} onClick={() => onNextClick()} tabIndex={2}>Create</Button>,
      ]}
      activeTimelineIndex={1}
      timelineItems={PartialDepositFlow}
      title="Top up a validator"
    >
      { creatingDeposit ? (
        <Loader message="Creating your deposit data file." />
      ) : (
        <div className="tw-flex tw-flex-col tw-items-center tw-gap-4">
          <Typography>Choose a folder where we should save your deposit data file.</Typography>

          <FolderSelector onFolderSelect={setSelectedFolder} />

          {selectedFolder && <Typography>You've selected: {selectedFolder}</Typography>}

          {generationError && <Typography color="error">{generationError}</Typography>}
        </div>
      )}
    </WizardWrapper>
  );
};

export default CreatePartialDeposit;
