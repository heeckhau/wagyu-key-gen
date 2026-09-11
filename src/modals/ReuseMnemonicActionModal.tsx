import { Button, Tooltip } from "@mui/material";

import { ReuseMnemonicAction } from "../types";
import WagyuModal from "./WagyuModal";

interface ReuseMnemonicActionModalParams {
  onClose: () => void;
  onSubmit: (action: ReuseMnemonicAction) => void;
  showModal: boolean;
}

/**
 * Modal for the user to pick which action they would like to take when reusing a mnemonic
 *
 * Options are: Generate existing keys, Generate BLS change, or sign a voluntary exit
 */
const ReuseMnemonicActionModal = ({ onClose, onSubmit, showModal}: ReuseMnemonicActionModalParams) => (
  <WagyuModal
    className="tw-w-[560px] tw-h-[320px]"
    open={showModal}
    onClose={onClose}
  >
    <div className="tw-flex tw-flex-col tw-h-full tw-my-7">
      <div className="tw-text-2xl">How would you like to use your existing secret recovery phrase?</div>

      <div className="tw-grow" />

      <div className="tw-flex tw-flex-col tw-gap-2 tw-align-middle">
        <div>
          <Button variant="contained" color="primary" onClick={() => onSubmit(ReuseMnemonicAction.RegenerateKeys)}>
            Generate existing or new validator keys
          </Button>
        </div>
        <div>
          <Tooltip title="If you initially created your validator keys without adding a withdrawal address, you can generate this BLS to execution change to add one once.">
            <Button variant="contained" color="primary" onClick={() => onSubmit(ReuseMnemonicAction.GenerateBLSToExecutionChange)}>
              Generate your BLS to execution change<br />(Add a withdrawal address)
            </Button>
          </Tooltip>
        </div>
        <div>
          <Tooltip title="Signs a voluntary exit for one or more of your validators. Once broadcast, a validator stops attesting and its balance is withdrawn. This cannot be undone.">
            <Button variant="contained" color="primary" onClick={() => onSubmit(ReuseMnemonicAction.GenerateExitTransaction)}>
              Exit your validator(s)
            </Button>
          </Tooltip>
        </div>
      </div>
    </div>
  </WagyuModal>
);

export default ReuseMnemonicActionModal;
