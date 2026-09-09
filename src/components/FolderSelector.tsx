import { Button, Typography } from "@mui/material";
import { useState } from "react";

import { bashUtils, electronAPI } from "../api";
import { errors } from "../constants";

interface FolderSelectorParams {
  onFolderSelect: (folder: string) => void;
}

/**
 * Component to select a folder and will use onFolderSelect param to provide
 * selected folder to parent
 * @param onFolderSelect callback to provide selected folder
 */
const FolderSelector = ({ onFolderSelect }: FolderSelectorParams) => {
  const [displayFolderPicker, setDisplayFolderPicker] = useState(false);
  const [errorMessage, setErrorMessage] = useState("");

  /**
   * Called upon selecting a folder. Will verify the folder exists and is writable
   * @param folderPath the path of the folder to verify
   */
  const verifyFolder = (folderPath: string) => {
    bashUtils.doesDirectoryExist(folderPath)
      .then((exists) => {
        if (!exists) {
          setErrorMessage(errors.FOLDER_DOES_NOT_EXISTS);
        } else {

          bashUtils.isDirectoryWritable(folderPath)
            .then((writable) => {
              if (!writable) {
                setErrorMessage(errors.FOLDER_IS_NOT_WRITABLE);
              } else {
                onFolderSelect(folderPath);
              }
            });
        }
      });
  };

  /**
   * Will open a dialog to allow the user to select a folder
   */
  const chooseFolder = () => {
    setErrorMessage("");
    setDisplayFolderPicker(true);

    electronAPI.invokeShowOpenDialog()
      .then((folder: string | null) => {
        if (folder) {
          verifyFolder(folder);
        }
      })
      .finally(() => {
        setDisplayFolderPicker(false);
      });
  };

  return (
    <div className="tw-flex tw-flex-col tw-gap-2">
      <div className="tw-text-center">
        <Button
          color="secondary"
          component="label"
          disabled={displayFolderPicker}
          onClick={chooseFolder}
          tabIndex={1}
          variant="contained"
        >
          Browse
        </Button>
      </div>
      {errorMessage && <Typography color="error">{errorMessage}</Typography>}
    </div>
  );
};

export default FolderSelector;
