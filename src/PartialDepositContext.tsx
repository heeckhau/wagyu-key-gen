import { Dispatch, SetStateAction, createContext, useState } from "react";

interface PartialDepositContextType {
  keystorePath: string;
  setKeystorePath: Dispatch<SetStateAction<string>>;
  keystorePassword: string;
  setKeystorePassword: Dispatch<SetStateAction<string>>;
  /** Deposit as typed by the user, in ETH (or GNO on Gnosis chains). */
  amount: string;
  setAmount: Dispatch<SetStateAction<string>>;
  withdrawalAddress: string;
  setWithdrawalAddress: Dispatch<SetStateAction<string>>;
  compounding: boolean;
  setCompounding: Dispatch<SetStateAction<boolean>>;
  folderLocation: string;
  setFolderLocation: Dispatch<SetStateAction<string>>;
}

export const PartialDepositContext = createContext<PartialDepositContextType>({
  keystorePath: "",
  setKeystorePath: () => {},
  keystorePassword: "",
  setKeystorePassword: () => {},
  amount: "",
  setAmount: () => {},
  withdrawalAddress: "",
  setWithdrawalAddress: () => {},
  compounding: false,
  setCompounding: () => {},
  folderLocation: "",
  setFolderLocation: () => {},
});

/**
 * Context for topping up an existing validator with a partial deposit
 */
const PartialDepositContextWrapper = ({ children }: { children: React.ReactNode }) => {
  const [keystorePath, setKeystorePath] = useState<string>("");
  const [keystorePassword, setKeystorePassword] = useState<string>("");
  const [amount, setAmount] = useState<string>("");
  const [withdrawalAddress, setWithdrawalAddress] = useState<string>("");
  const [compounding, setCompounding] = useState<boolean>(false);
  const [folderLocation, setFolderLocation] = useState<string>("");

  return (
    <PartialDepositContext.Provider value={{
      keystorePath,
      setKeystorePath,
      keystorePassword,
      setKeystorePassword,
      amount,
      setAmount,
      withdrawalAddress,
      setWithdrawalAddress,
      compounding,
      setCompounding,
      folderLocation,
      setFolderLocation,
    }}>
      {children}
    </PartialDepositContext.Provider>
  );
};

export default PartialDepositContextWrapper;
