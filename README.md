# Wagyu Key Gen
[![gitpoap badge](https://public-api.gitpoap.io/v1/repo/stake-house/wagyu-key-gen/badge)](https://www.gitpoap.io/gh/stake-house/wagyu-key-gen)

Wagyu Key Gen is a GUI application for creating Ethereum validator keys: it generates a Secret Recovery Phrase (mnemonic), EIP-2335 keystores and `deposit_data` files, and for existing validators it produces BLS-to-execution-change files, signed voluntary exits and top-up (partial) deposits, from the mnemonic or from a keystore file. It writes exactly the same files as the [ethstaker-deposit-cli](https://github.com/eth-educators/ethstaker-deposit-cli), and ships a [command-line front end](#command-line) with that CLI's commands.

Since version 2.0 the application is a React UI running in [Tauri](https://tauri.app) with a Rust backend. All cryptography (BLS12-381, EIP-2333 key derivation, EIP-2335 keystores, BIP-39) comes from the [Lighthouse](https://github.com/sigp/lighthouse) crypto crates.

### Download wagyu at [https://wagyu.gg](https://wagyu.gg)

### Wagyu Audit by HashCloak [Wagyu Key Gen Audit Report](https://github.com/stake-house/wagyu-key-gen/files/7693548/Wagyu.Key.Gen.Audit.Report.pdf)

The audit covered the Electron + Python implementation (versions 1.x). The Rust rewrite (2.x) reproduces the audited output byte for byte (see the golden fixtures under `crates/wagyu-core/tests/vectors/golden`) but has not itself been audited yet.

## Why the rewrite?

Wagyu 1.x was Electron + a Python subprocess. Issues that motivated the move to a single Rust/Tauri binary:

- **Secrets crossed a process boundary as CLI arguments.** The Electron app shelled out to a Python script with the mnemonic password on the command line, visible to anything reading the process list, and the likely cause of [#188](https://github.com/stake-house/wagyu-key-gen/issues/188) (pasted passwords breaking key generation). The Rust core runs in-process: no subprocess, no argv, no parsing layer between UI and crypto.
- **Perpetual dependency-CVE chasing.** The [HashCloak audit](https://github.com/stake-house/wagyu-key-gen/files/7693548/Wagyu.Key.Gen.Audit.Report.pdf) (Nov 2021) flagged out-of-date/unused npm dependencies and `nodeIntegration: true`; a high-severity `web3-utils` prototype-pollution CVE ([#190](https://github.com/stake-house/wagyu-key-gen/issues/190)) is still open in the npm tree, and a maintainer called Electron itself "too old and potentially dangerous" in [#181](https://github.com/stake-house/wagyu-key-gen/issues/181). The Rust core depends on a small, audited set of Lighthouse crypto crates instead of a large npm graph.
- **Packaging/signing kept breaking.** [#217](https://github.com/stake-house/wagyu-key-gen/issues/217) and [#200](https://github.com/stake-house/wagyu-key-gen/issues/200) are recurring macOS "app is damaged" / corrupted-build reports, the same failure class this migration hit and fixed with ad-hoc signing (`680fa13`).
- **Electron-specific bugs with no equivalent in a native shell.** The old app froze entirely when opening a folder dialog on some Linux live environments ([#144](https://github.com/stake-house/wagyu-key-gen/issues/144)), and running it could hijack global keyboard shortcuts in unrelated apps ([#148](https://github.com/stake-house/wagyu-key-gen/issues/148)).
- **Secrets can be deterministically wiped from memory.** Mnemonics and passwords are wrapped end to end in [`zeroize::Zeroizing`](https://crates.io/crates/zeroize) (`mnemonic.rs`, `deposit.rs`, `btec.rs`), which zeroes the underlying memory as soon as the value is dropped. A garbage-collected runtime — Electron's Node/V8, or the old Python subprocess — can't make that guarantee: strings are immutable and stray copies can sit on the heap indefinitely, so scrubbing them is best-effort. The clipboard gets the same treatment: the app clears it on exit so a copied mnemonic doesn't outlive the process (`883af7c`).
- **No bundled Chromium/Node runtime.** Electron ships its own copy of both in every install; Tauri uses the OS's native webview plus a Rust binary. The current build's macOS `.dmg` is 3.8MB (23MB unpacked) — Electron's runtime alone typically costs 80MB+ before any application code, on every platform, for every install.

## Repository layout

| Path | Contents |
|---|---|
| `crates/wagyu-core` | Rust library with all key handling: mnemonics, credentials, keystores, deposit data, BLS-to-execution changes, voluntary exits, partial deposits. No UI dependencies, fully unit tested. |
| `crates/wagyu-cli` | Terminal front end for `wagyu-core` with the commands and options of ethstaker-deposit-cli. |
| `src-tauri` | The Tauri application: window, IPC commands, plugins, bundling configuration. |
| `src` | The React + MUI + Tailwind user interface. `src/api` is the bridge to the Rust backend. |

## Development setup

You need:

- [Rust](https://rustup.rs) stable (1.85 or newer; `rust-toolchain.toml` selects it automatically).
- [Node.js](https://nodejs.org) 20 or newer (npm ships with it).
- The Tauri platform prerequisites for your OS: https://v2.tauri.app/start/prerequisites/
  - macOS: Xcode command line tools (`xcode-select --install`).
  - Ubuntu / Debian: `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf build-essential curl wget file libssl-dev`
  - Windows: Microsoft C++ Build Tools and the WebView2 runtime (already present on Windows 11 and updated Windows 10).

Then:

```console
git clone https://github.com/stake-house/wagyu-key-gen
cd wagyu-key-gen
npm install
```

## Start Wagyu Key Gen

```console
npm run tauri dev
```

This starts Vite for the UI with hot reload and compiles the Rust backend. The first compile takes a few minutes (Lighthouse and Tauri are large); later ones are incremental.

To open the web inspector in a dev build use `Ctrl` + `Shift` + `I` (`Cmd` + `Option` + `I` on macOS). Release builds have the inspector disabled.

## Tests

```console
cargo test --workspace   # Rust: BIP-39 / EIP-2333 / EIP-2335 vectors, every generator's parity with the Python deposit-cli output, the Tauri IPC layer, the CLI end to end
npm test                 # UI unit tests (Vitest)
```

The golden fixtures under `crates/wagyu-core/tests/vectors/golden` were produced by ethstaker-deposit-cli 1.2.2 itself; each `params.json` records the inputs and the sibling files are the expected output.

## Bundling

```console
npm run tauri build
```

On x86_64 machines add `--features portable` so the BLS backend does not require ADX instructions (older CPUs would otherwise crash):

```console
npm run tauri build -- --features portable
```

The installers land in `target/release/bundle/` (`dmg/` on macOS, `appimage/` on Linux, `nsis/` on Windows). On Windows `target/release/wagyu-key-gen.exe` is also usable as a portable executable on machines that already have the WebView2 runtime.

Release binaries are produced by the `ci-build` GitHub workflow (`.github/workflows/build.yml`).

## Command line

`wagyu-cli` drives the same Rust core from a terminal, with the commands and `--snake_case` options of [ethstaker-deposit-cli](https://github.com/eth-educators/ethstaker-deposit-cli) so existing guides and scripts carry over:

```console
cargo build --release -p wagyu-cli      # add `--features portable` on x86_64, as for the app
./target/release/wagyu-cli --help

./target/release/wagyu-cli new-mnemonic --chain mainnet --num_validators 2
./target/release/wagyu-cli existing-mnemonic --chain hoodi --validator_start_index 4 --num_validators 1
./target/release/wagyu-cli generate-bls-to-execution-change --chain mainnet --validator_indices 1,2 \
    --bls_withdrawal_credentials_list 0x00...,0x00... --withdrawal_address 0x...
./target/release/wagyu-cli exit-transaction-mnemonic --chain mainnet --validator_indices 1,2
./target/release/wagyu-cli exit-transaction-keystore --keystore validator_keys/keystore-m_12381_3600_0_0_0-1.json --validator_index 1
./target/release/wagyu-cli partial-deposit --keystore validator_keys/keystore-m_12381_3600_0_0_0-1.json --amount 1 \
    --withdrawal_address 0x... --compounding
./target/release/wagyu-cli generate-bls-to-execution-change-keystore --keystore ... --validator_index 1 --withdrawal_address 0x...
```

Secrets (the mnemonic, keystore passwords) are prompted for when their option is omitted, so they stay out of the shell history; `--non_interactive` disables every prompt for scripting. Files go to the same subfolders as the Python CLI (`validator_keys/`, `bls_to_execution_changes/`, `bls_to_execution_changes_keystore/`, `exit_transactions/`, `partial_deposits/`) with the same names and contents; `crates/wagyu-cli/tests/cli.rs` checks them against the golden fixtures.

Differences from ethstaker-deposit-cli 1.2.2: prompts are English only, `--devnet_chain_setting` is not supported, `--non_interactive new-mnemonic` prints the mnemonic without asking for it back, and `partial-deposit --amount` is in GNO on Gnosis chains and multiplied by 32 like `new-mnemonic`/`existing-mnemonic` do (the Python `partial-deposit` does not apply the multiplier).

## Design
Current designs: https://www.figma.com/file/jcF78fVjndvM2hOPvifl0N/Wagyu-Key?node-id=1%3A4

## Funding

If you would like to help us with funding this project, you can donate with our [Gitcoin grant](https://gitcoin.co/grants/2112/stakehouse-wagyu-tooling-suite-easy-to-use-tools-) or you can send your funds directly to `wagyutools.eth`.

## Support
Reach out to the EthStaker community:
 - on [discord](https://dsc.gg/ethstaker)
 - on [reddit](https://www.reddit.com/r/ethstaker/)

## License
[GPL](LICENSE)
