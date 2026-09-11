import { Button, Link, Typography } from "@mui/material";
import { useContext, useEffect } from "react";
import { useHistory } from "react-router-dom";

import { bashUtils, electronAPI } from "../api";
import WizardWrapper from "../components/WizardWrapper";
import { PartialDepositFlow } from "../constants";
import { PartialDepositContext } from "../PartialDepositContext";

/**
 * Final step of the top-up flow. Shows where the deposit data was saved and how to use it.
 */
const FinishPartialDeposit = () => {
  const { folderLocation } = useContext(PartialDepositContext);
  const history = useHistory();

  useEffect(() => {
    if (!folderLocation) {
      history.replace("/");
    }

    // On browser back, go back to main page and refresh to clear navigation history
    const unblock = history.block(() => {
      unblock();
      history.push("/");
      window.location.reload();
    });
  }, []);

  const openFileLocation = () => {
    bashUtils.findFirstFile(folderLocation, "deposit_data")
      .then((depositFile) => {
        electronAPI.shellShowItemInFolder(depositFile !== "" ? depositFile : folderLocation);
      });
  };

  const onClose = () => {
    electronAPI.ipcRendererSendClose();
  };

  return (
    <WizardWrapper
      actionBarItems={[<Button variant="contained" color="primary" onClick={() => onClose()} tabIndex={2}>Close</Button>]}
      activeTimelineIndex={2}
      timelineItems={PartialDepositFlow}
      title="Top up a validator"
    >
      <div className="tw-flex tw-flex-col tw-mx-28">
        <Typography variant="body1" align="left">
          Your deposit data file has been created here:{" "}
          <Link
            display="inline"
            component="button"
            onClick={openFileLocation}
          >
            {folderLocation}
          </Link>
        </Typography>

        <Typography className="tw-mt-16" variant="body1">
          There is a single file for this:
        </Typography>
        <Typography className="tw-text-cyan">
          Deposit data file (ex. deposit_data-xxxxxx.json)
        </Typography>
        <Typography variant="body2">
          This file represents public information about the top-up deposit for your existing validator. Submit it through the Ethereum Launchpad, exactly like the deposit data of a new validator, to make the deposit.
        </Typography>
        <Typography className="tw-text-gray">
          Note: Your clipboard will be cleared upon closing this application.
        </Typography>
      </div>
    </WizardWrapper>
  );
};

export default FinishPartialDeposit;
