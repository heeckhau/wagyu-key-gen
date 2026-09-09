import { Typography } from "@mui/material";
import { useEffect, useState } from "react";

import { electronAPI, VersionInfo } from "../api";

/**
 * Footer to display the version and commit hash, provided by the backend at build time.
 */
const VersionFooter = () => {
  const [info, setInfo] = useState<VersionInfo>({ version: "", commit: "" });

  useEffect(() => {
    electronAPI.versionInfo().then(setInfo).catch(() => {});
  }, []);

  return (
    <div className="tw-fixed tw-bottom-9 tw-w-full">
      <Typography className="tw-text-gray tw-text-center tw-text-xxs">Version: {info.version} - Commit Hash: {info.commit}</Typography>
    </div>
  );
};

export default VersionFooter;
