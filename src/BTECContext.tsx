import { Dispatch, SetStateAction, createContext, useState } from "react";

import { KeySource } from "./types";

interface BTECContextType {
  /** Whether the change is signed with keys from the mnemonic or from a keystore file. */
  source: KeySource;
  setSource: Dispatch<SetStateAction<KeySource>>;
  btecCredentials: string;
  setBTECCredentials: Dispatch<SetStateAction<string>>;
  btecIndices: string;
  setBTECIndices: Dispatch<SetStateAction<string>>;
  folderLocation: string;
  setFolderLocation: Dispatch<SetStateAction<string>>;
  index: number;
  setIndex: Dispatch<SetStateAction<number>>;
  mnemonic: string;
  setMnemonic: Dispatch<SetStateAction<string>>;
  withdrawalAddress: string;
  setWithdrawalAddress: Dispatch<SetStateAction<string>>;
  keystorePath: string;
  setKeystorePath: Dispatch<SetStateAction<string>>;
  keystorePassword: string;
  setKeystorePassword: Dispatch<SetStateAction<string>>;
  /** Beacon-chain index of the validator in the keystore (keystore source only). */
  validatorIndex: number;
  setValidatorIndex: Dispatch<SetStateAction<number>>;
}

export const BTECContext = createContext<BTECContextType>({
  source: "mnemonic",
  setSource: () => {},
  btecCredentials: "",
  setBTECCredentials: () => {},
  btecIndices: "",
  setBTECIndices: () => {},
  folderLocation: "",
  setFolderLocation: () => {},
  index: 0,
  setIndex: () => {},
  mnemonic: "",
  setMnemonic: () => {},
  withdrawalAddress: "",
  setWithdrawalAddress: () => {},
  keystorePath: "",
  setKeystorePath: () => {},
  keystorePassword: "",
  setKeystorePassword: () => {},
  validatorIndex: 0,
  setValidatorIndex: () => {},
});

/**
 * Context for making the withdrawal credentials change
 */
const BTECContextWrapper = ({ children }: { children: React.ReactNode}) => {
  const [source, setSource] = useState<KeySource>("mnemonic");
  const [btecIndices, setBTECIndices] = useState<string>("");
  const [btecCredentials, setBTECCredentials] = useState<string>("");
  const [folderLocation, setFolderLocation] = useState<string>("");
  const [index, setIndex] = useState<number>(0);
  const [mnemonic, setMnemonic] = useState<string>("");
  const [withdrawalAddress, setWithdrawalAddress] = useState<string>("");
  const [keystorePath, setKeystorePath] = useState<string>("");
  const [keystorePassword, setKeystorePassword] = useState<string>("");
  const [validatorIndex, setValidatorIndex] = useState<number>(0);

  return (
    <BTECContext.Provider value={{
      source,
      setSource,
      btecCredentials,
      setBTECCredentials,
      btecIndices,
      setBTECIndices,
      folderLocation,
      setFolderLocation,
      index,
      setIndex,
      mnemonic,
      setMnemonic,
      withdrawalAddress,
      setWithdrawalAddress,
      keystorePath,
      setKeystorePath,
      keystorePassword,
      setKeystorePassword,
      validatorIndex,
      setValidatorIndex,
    }}>
      {children}
    </BTECContext.Provider>
  );
};

export default BTECContextWrapper;
