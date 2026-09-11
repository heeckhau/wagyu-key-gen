import { CssBaseline, ThemeProvider, StyledEngineProvider } from "@mui/material";
import { FC, ReactElement } from "react";
import { HashRouter, Route, Switch } from "react-router-dom";

import BTECContextWrapper from "./BTECContext";
import { OnlineDetector } from "./components/OnlineDetector";
import VersionFooter from "./components/VersionFooter";
import { paths } from "./constants";
import ExitContextWrapper from "./ExitContext";
import GlobalContextWrapper from "./GlobalContext";
import KeyCreationContextWrapper from "./KeyCreationContext";
import ConfigureBTECKeystore from "./pages/ConfigureBTECKeystore";
import ConfigureExitKeystore from "./pages/ConfigureExitKeystore";
import ConfigureExitTransaction from "./pages/ConfigureExitTransaction";
import ConfigurePartialDeposit from "./pages/ConfigurePartialDeposit";
import ConfigureValidatorKeys from "./pages/ConfigureValidatorKeys";
import ConfigureWithdrawalAddress from "./pages/ConfigureWithdrawalAddress";
import CreateCredentialsChange from "./pages/CreateCredentialsChange";
import CreateExitTransaction from "./pages/CreateExitTransaction";
import CreateMnemonic from "./pages/CreateMnemonic";
import CreatePartialDeposit from "./pages/CreatePartialDeposit";
import CreateValidatorKeys from "./pages/CreateValidatorKeys";
import FinishCredentialsGeneration from "./pages/FinishCredentialsGeneration";
import FinishExitTransaction from "./pages/FinishExitTransaction";
import FinishKeyGeneration from "./pages/FinishKeyGeneration";
import FinishPartialDeposit from "./pages/FinishPartialDeposit";
import Home from "./pages/Home";
import MnemonicImport from "./pages/MnemonicImport";
import PartialDepositContextWrapper from "./PartialDepositContext";
import theme from "./theme";

/**
 * Routing for the application. Broken into sections:
 * - Primary home page
 * - Routes for creating a mnemonic and validator keys
 * - Routes for using an existing mnemonic to create validator keys
 * - Routes for generating the credentials change (from a mnemonic or a keystore)
 * - Routes for generating exit transactions (from a mnemonic or a keystore)
 * - Routes for topping up a validator from a keystore
 *
 * Each flow is wrapped in a React Context that will store
 * the inputs of the user to be accessible across each page. This prevents
 * prop drilling
 */
const App: FC = (): ReactElement => {
  return (
    <StyledEngineProvider injectFirst>
      <ThemeProvider theme={theme}>
        <CssBaseline />
        <HashRouter>
          <GlobalContextWrapper>
            <main className="tw-flex tw-flex-col tw-h-[100vh]">
              <OnlineDetector />
              <Switch>
                <Route exact path="/" render={() => <Home />} />

                <Route>

                  {/* Create Mnemonic & Keys Flow */}
                  <KeyCreationContextWrapper>
                    <Switch>
                      <Route path={paths.CREATE_MNEMONIC} children={() => <CreateMnemonic />} />
                      <Route path={paths.CONFIGURE_CREATE} children={() => <ConfigureValidatorKeys />} />
                      <Route path={paths.CREATE_KEYS_CREATE} children={() => <CreateValidatorKeys />} />
                      <Route path={paths.FINISH_CREATE} children={() => <FinishKeyGeneration />} />
                    </Switch>
                  </KeyCreationContextWrapper>


                  {/* Import Mnemonic & Generate Keys Flow */}
                  <KeyCreationContextWrapper>
                    <Switch>
                      <Route path={paths.EXISTING_IMPORT} render={() => <MnemonicImport />} />
                      <Route path={paths.CONFIGURE_EXISTING} render={() => <ConfigureValidatorKeys />} />
                      <Route path={paths.CREATE_KEYS_EXISTING} render={() => <CreateValidatorKeys />} />
                      <Route path={paths.FINISH_EXISTING} render={() => <FinishKeyGeneration />} />
                    </Switch>
                  </KeyCreationContextWrapper>


                  {/* Update Withdrawal Credentials Flow */}
                  <BTECContextWrapper>
                    <Switch>
                      <Route path={paths.BTEC_IMPORT} render={() => <MnemonicImport />} />
                      <Route path={paths.CONFIGURE_BTEC} render={() => <ConfigureWithdrawalAddress />} />
                      <Route path={paths.CONFIGURE_BTEC_KEYSTORE} render={() => <ConfigureBTECKeystore />} />
                      <Route path={paths.CREATE_CREDENTIALS} render={() => <CreateCredentialsChange />} />
                      <Route path={paths.FINISH_CREDENTIALS} render={() => <FinishCredentialsGeneration />} />
                    </Switch>
                  </BTECContextWrapper>


                  {/* Exit Transaction Flow */}
                  <ExitContextWrapper>
                    <Switch>
                      <Route path={paths.EXIT_IMPORT} render={() => <MnemonicImport />} />
                      <Route path={paths.CONFIGURE_EXIT} render={() => <ConfigureExitTransaction />} />
                      <Route path={paths.CONFIGURE_EXIT_KEYSTORE} render={() => <ConfigureExitKeystore />} />
                      <Route path={paths.CREATE_EXIT} render={() => <CreateExitTransaction />} />
                      <Route path={paths.FINISH_EXIT} render={() => <FinishExitTransaction />} />
                    </Switch>
                  </ExitContextWrapper>


                  {/* Partial Deposit (Top Up) Flow */}
                  <PartialDepositContextWrapper>
                    <Switch>
                      <Route path={paths.CONFIGURE_PARTIAL_DEPOSIT} render={() => <ConfigurePartialDeposit />} />
                      <Route path={paths.CREATE_PARTIAL_DEPOSIT} render={() => <CreatePartialDeposit />} />
                      <Route path={paths.FINISH_PARTIAL_DEPOSIT} render={() => <FinishPartialDeposit />} />
                    </Switch>
                  </PartialDepositContextWrapper>
                </Route>
              </Switch>
              <VersionFooter />
            </main>
          </GlobalContextWrapper>
        </HashRouter>
      </ThemeProvider>
    </StyledEngineProvider>
  );
};

export default App;
