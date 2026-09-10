import { Button, Link, Typography } from "@mui/material";
import { useContext, useEffect } from "react";
import { useHistory } from "react-router-dom";

import { bashUtils, electronAPI } from "../api";
import WizardWrapper from "../components/WizardWrapper";
import { ExitKeystoreFlow, ExitMnemonicFlow } from "../constants";
import { ExitContext } from "../ExitContext";

/**
 * Final step of the exit flow. Shows where the signed exit(s) were saved and how to use them.
 */
const FinishExitTransaction = () => {
  const { folderLocation, source } = useContext(ExitContext);
  const history = useHistory();
  const usingKeystore = source === "keystore";

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
    bashUtils.findFirstFile(folderLocation, "signed_exit_transaction")
      .then((exitFile) => {
        electronAPI.shellShowItemInFolder(exitFile !== "" ? exitFile : folderLocation);
      });
  };

  const onClose = () => {
    electronAPI.ipcRendererSendClose();
  };

  return (
    <WizardWrapper
      actionBarItems={[<Button variant="contained" color="primary" onClick={() => onClose()} tabIndex={2}>Close</Button>]}
      activeTimelineIndex={usingKeystore ? 2 : 3}
      timelineItems={usingKeystore ? ExitKeystoreFlow : ExitMnemonicFlow}
      title="Generate exit transaction"
    >
      <div className="tw-flex tw-flex-col tw-mx-28">
        <Typography variant="body1" align="left">
          Your signed exit transaction file(s) have been created here:{" "}
          <Link
            display="inline"
            component="button"
            onClick={openFileLocation}
          >
            {folderLocation}
          </Link>
        </Typography>

        <Typography className="tw-mt-16" variant="body1">
          There is one file per validator:
        </Typography>
        <Typography className="tw-text-cyan">
          Signed exit transaction (ex. signed_exit_transaction-xxxxx-xxxxxxx.json)
        </Typography>
        <Typography variant="body2">
          This file contains your validator's signed request to leave the network. Nothing happens until you broadcast it, for example with the <em>Broadcast Signed Messages</em> tool on the beaconcha.in website. Once broadcast the exit cannot be cancelled: the validator stops attesting and, after the exit is processed, its balance is withdrawn to its withdrawal address.
        </Typography>
        <Typography className="tw-text-gray">
          Note: Your clipboard will be cleared upon closing this application.
        </Typography>
      </div>
    </WizardWrapper>
  );
};

export default FinishExitTransaction;
