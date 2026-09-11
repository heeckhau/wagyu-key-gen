import { Button, Link, Typography } from "@mui/material";
import { useContext, useEffect } from "react";
import { useHistory } from "react-router-dom";

import { bashUtils, electronAPI } from "../api";
import { BTECContext } from "../BTECContext";
import WizardWrapper from "../components/WizardWrapper";
import { BTECFlow, BTECKeystoreFlow } from "../constants";

/**
 * Final step of the credentials generation flow.
 * Shows the user the location where the files were stored and provides
 * some additional information.
 */
const FinishCredentialsGeneration = () => {
  const { folderLocation, source } = useContext(BTECContext);
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

  /**
   * Will open a directory explorer where the credential change file was saved
   */
  const openKeyLocation = () => {
    bashUtils.findFirstFile(folderLocation, "bls_to_execution_change")
      .then((changeFile) => {
        let fileToLocate = folderLocation;
        if (changeFile !== "") {
          fileToLocate = changeFile;
        }
        electronAPI.shellShowItemInFolder(fileToLocate);
    });
  };

  const onClose = () => {
    electronAPI.ipcRendererSendClose();
  };

  return (
    <WizardWrapper
      actionBarItems={[<Button variant="contained" color="primary" onClick={() => onClose()} tabIndex={2}>Close</Button>]}
      activeTimelineIndex={usingKeystore ? 2 : 3}
      timelineItems={usingKeystore ? BTECKeystoreFlow : BTECFlow}
      title="Generate BLS to execution change"
    >
      <div className="tw-flex tw-flex-col tw-mx-28">
        <Typography variant="body1" align="left">
          Your BLS to execution change file has been created here:{" "}
          <Link
            display="inline"
            component="button"
            onClick={openKeyLocation}
          >
            {folderLocation}
          </Link>
        </Typography>


        <Typography className="tw-mt-16" variant="body1">
          There is a single file for this:
        </Typography>
        <Typography className="tw-text-cyan">
          {usingKeystore
            ? "BLS to execution keystore signature (ex. bls_to_execution_change_keystore_signature-x-xxxxxxx.json)"
            : "BLS to execution file (ex. bls_to_execution_change-xxxxxxx.json)"}
        </Typography>
        <Typography variant="body2">
          {usingKeystore
            ? "This file contains your validator's signature, made with its signing key, authorising the withdrawal address. It is meant for services that accept keystore-signed changes; it is not the standard message that beaconcha.in broadcasts."
            : <>This file contains your signature to add your withdrawal address on your validator(s). You can easily publish it on beaconcha.in website by using their <em>Broadcast Signed Messages</em> tool.</>}
        </Typography>
        <Typography className="tw-text-gray">
          Note: Your clipboard will be cleared upon closing this application.
        </Typography>
      </div>
    </WizardWrapper>
  )
};

export default FinishCredentialsGeneration;
