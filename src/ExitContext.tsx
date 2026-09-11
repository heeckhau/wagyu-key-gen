import { Dispatch, SetStateAction, createContext, useState } from "react";

import { KeySource } from "./types";

interface ExitContextType {
  /** Whether the exit is signed with keys from the mnemonic or from a keystore file. */
  source: KeySource;
  setSource: Dispatch<SetStateAction<KeySource>>;
  mnemonic: string;
  setMnemonic: Dispatch<SetStateAction<string>>;
  /** EIP-2334 index of the first key (mnemonic source only). */
  index: number;
  setIndex: Dispatch<SetStateAction<number>>;
  /** Comma separated beacon-chain validator indices (mnemonic source only). */
  indices: string;
  setIndices: Dispatch<SetStateAction<string>>;
  keystorePath: string;
  setKeystorePath: Dispatch<SetStateAction<string>>;
  keystorePassword: string;
  setKeystorePassword: Dispatch<SetStateAction<string>>;
  /** Beacon-chain index of the validator in the keystore (keystore source only). */
  validatorIndex: number;
  setValidatorIndex: Dispatch<SetStateAction<number>>;
  epoch: number;
  setEpoch: Dispatch<SetStateAction<number>>;
  folderLocation: string;
  setFolderLocation: Dispatch<SetStateAction<string>>;
}

export const ExitContext = createContext<ExitContextType>({
  source: "mnemonic",
  setSource: () => {},
  mnemonic: "",
  setMnemonic: () => {},
  index: 0,
  setIndex: () => {},
  indices: "",
  setIndices: () => {},
  keystorePath: "",
  setKeystorePath: () => {},
  keystorePassword: "",
  setKeystorePassword: () => {},
  validatorIndex: 0,
  setValidatorIndex: () => {},
  epoch: 0,
  setEpoch: () => {},
  folderLocation: "",
  setFolderLocation: () => {},
});

/**
 * Context for generating signed voluntary exits
 */
const ExitContextWrapper = ({ children }: { children: React.ReactNode }) => {
  const [source, setSource] = useState<KeySource>("mnemonic");
  const [mnemonic, setMnemonic] = useState<string>("");
  const [index, setIndex] = useState<number>(0);
  const [indices, setIndices] = useState<string>("");
  const [keystorePath, setKeystorePath] = useState<string>("");
  const [keystorePassword, setKeystorePassword] = useState<string>("");
  const [validatorIndex, setValidatorIndex] = useState<number>(0);
  const [epoch, setEpoch] = useState<number>(0);
  const [folderLocation, setFolderLocation] = useState<string>("");

  return (
    <ExitContext.Provider value={{
      source,
      setSource,
      mnemonic,
      setMnemonic,
      index,
      setIndex,
      indices,
      setIndices,
      keystorePath,
      setKeystorePath,
      keystorePassword,
      setKeystorePassword,
      validatorIndex,
      setValidatorIndex,
      epoch,
      setEpoch,
      folderLocation,
      setFolderLocation,
    }}>
      {children}
    </ExitContext.Provider>
  );
};

export default ExitContextWrapper;
