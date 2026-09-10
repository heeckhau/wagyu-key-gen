# Wagyu Key Gen
[![gitpoap badge](https://public-api.gitpoap.io/v1/repo/stake-house/wagyu-key-gen/badge)](https://www.gitpoap.io/gh/stake-house/wagyu-key-gen)

Wagyu Key Gen is a GUI application for creating Ethereum validator keys: it generates a Secret Recovery Phrase (mnemonic), EIP-2335 keystores and `deposit_data` files, and can produce BLS-to-execution-change files for an existing mnemonic. It writes exactly the same files as the [ethstaker-deposit-cli](https://github.com/eth-educators/ethstaker-deposit-cli).

Since version 2.0 the application is a React UI running in [Tauri](https://tauri.app) with a Rust backend. All cryptography (BLS12-381, EIP-2333 key derivation, EIP-2335 keystores, BIP-39) comes from the [Lighthouse](https://github.com/sigp/lighthouse) crypto crates.

### Download wagyu at [https://wagyu.gg](https://wagyu.gg)

### Wagyu Audit by HashCloak [Wagyu Key Gen Audit Report](https://github.com/stake-house/wagyu-key-gen/files/7693548/Wagyu.Key.Gen.Audit.Report.pdf)

The audit covered the Electron + Python implementation (versions 1.x). The Rust rewrite (2.x) reproduces the audited output byte for byte (see the golden fixtures under `crates/wagyu-core/tests/vectors/golden`) but has not itself been audited yet.

## Repository layout

| Path | Contents |
|---|---|
| `crates/wagyu-core` | Rust library with all key handling: mnemonics, credentials, keystores, deposit data, BLS-to-execution changes. No UI dependencies, fully unit tested. |
| `src-tauri` | The Tauri application: window, IPC commands, plugins, bundling configuration. |
| `src` | The React + MUI + Tailwind user interface. `src/api` is the bridge to the Rust backend. |
| `docs` | Design documents. |

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
cargo test --workspace   # Rust: BIP-39 / EIP-2333 / EIP-2335 vectors, deposit and BTEC generation, parity with the Python deposit-cli output
npm test                 # UI unit tests (Vitest)
```

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
