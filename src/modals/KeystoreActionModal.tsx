import { Button, Tooltip } from "@mui/material";

import { KeystoreAction } from "../types";
import WagyuModal from "./WagyuModal";

interface KeystoreActionModalParams {
  onClose: () => void;
  onSubmit: (action: KeystoreAction) => void;
  showModal: boolean;
}

/**
 * Modal for the user to pick what to do with an existing keystore file
 *
 * Options are: sign a voluntary exit, top up the validator, or add a withdrawal address
 */
const KeystoreActionModal = ({ onClose, onSubmit, showModal }: KeystoreActionModalParams) => (
  <WagyuModal
    className="tw-w-[560px] tw-h-[320px]"
    open={showModal}
    onClose={onClose}
  >
    <div className="tw-flex tw-flex-col tw-h-full tw-my-7">
      <div className="tw-text-2xl">What would you like to do with your keystore file?</div>

      <div className="tw-grow" />

      <div className="tw-flex tw-flex-col tw-gap-2 tw-align-middle">
        <div>
          <Tooltip title="Signs a voluntary exit for the validator in the keystore. Once broadcast, the validator stops attesting and its balance is withdrawn. This cannot be undone.">
            <Button variant="contained" color="primary" onClick={() => onSubmit(KeystoreAction.GenerateExitTransaction)}>
              Exit a validator
            </Button>
          </Tooltip>
        </div>
        <div>
          <Tooltip title="Creates a deposit data file that adds more ETH to the validator's balance.">
            <Button variant="contained" color="primary" onClick={() => onSubmit(KeystoreAction.GeneratePartialDeposit)}>
              Top up a validator
            </Button>
          </Tooltip>
        </div>
        <div>
          <Tooltip title="If you initially created your validator keys without adding a withdrawal address, you can generate this BLS to execution change to add one once, signed with the keystore instead of your Secret Recovery Phrase.">
            <Button variant="contained" color="primary" onClick={() => onSubmit(KeystoreAction.GenerateBLSToExecutionChange)}>
              Generate your BLS to execution change<br />(Add a withdrawal address)
            </Button>
          </Tooltip>
        </div>
      </div>
    </div>
  </WagyuModal>
);

export default KeystoreActionModal;
