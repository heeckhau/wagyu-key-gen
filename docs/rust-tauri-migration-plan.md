# Wagyu Key Gen: migration plan to Rust + Tauri

Status: implemented on 2026-09-09 (phases 0 to 6). Phase 7 extras: any-language mnemonic
import and the PBKDF2 keystore option exist in the core and the IPC layer; the UI toggle for
PBKDF2, progress events and exit transactions are still open. Section 12 lists what changed
from the plan while implementing.

## 1. Goal and constraints

Replace the Electron shell and the PyInstaller-bundled Python proxy around
`ethstaker-deposit-cli` with a Tauri 2 desktop app whose backend is a Rust
library. The React UI is kept; only its bridge to the backend changes.

Hard constraints agreed with the maintainer:

- **No hand-written cryptography.** BLS12-381, EIP-2333 key derivation,
  EIP-2335 keystores, BIP-39 seed derivation, scrypt/PBKDF2/AES all come from
  Lighthouse crates or the `bip39` crate. The only "spec" code written here is
  SSZ container layouts (derive macros) and domain/signing-root helpers, which
  are hashing-structure code, not cipher/KDF/curve code.
- **Output files stay byte-compatible** with what ethstaker-deposit-cli 1.2.2
  produces (deposit_data JSON, EIP-2335 keystore JSON, bls_to_execution_change
  JSON). The Ethereum launchpad and clients consume these.
- **Tests are ported** wherever they still test something we own; tests of
  Python internals or of features Wagyu does not expose are dropped and listed.
- **Frontend**: keep React + MUI + Tailwind; swap webpack for Vite; replace the
  `window.electronAPI` / `window.eth2Deposit` / `window.bashUtils` /
  `window.web3Utils` bridge with a thin module over Tauri `invoke` + plugins.
- **Scope**: feature parity plus low-cost extras that the Rust core gets almost
  for free (PBKDF2 keystore option, any-language mnemonic import, mnemonic
  language picker, exit transactions as a later optional phase).
- **Windows WebView2**: Tauri default `downloadBootstrapper`.

## 2. What the current app actually does (inventory)

| Layer | File(s) | Responsibility |
|---|---|---|
| Electron main | `src/electron/index.ts` | Window 950x750, hides menu, denies all permission requests, IPC handlers, clears clipboard on quit, title `Wagyu Key Gen {VERSION}-{COMMITHASH}` |
| Bridge | `src/electron/preload.ts`, `renderer.d.ts` | Exposes 4 namespaces to the renderer (listed in section 5) |
| CLI wrapper | `src/electron/Eth2Deposit.ts` | Spawns `stakingdeposit_proxy` (bundled SFE, local SFE, or system python) with 5 subcommands |
| FS helpers | `src/electron/BashUtils.ts` | `doesDirectoryExist`, `isDirectoryWritable` (writes a temp file), `findFirstFile` |
| Python proxy | `src/scripts/stakingdeposit_proxy.py` | `create_mnemonic`, `validate_mnemonic`, `generate_keys`, `validate_bls_credentials`, `bls_change`; parallelised with `ProcessPoolExecutor` |
| Vendored CLI | `src/vendors/ethstaker-deposit-cli-1.2.2` | Key derivation, keystores, SSZ, chain settings, validation, plus its test-suite and vectors |
| UI | `src/react/**` | 3 wizard flows (create mnemonic, import mnemonic, BLS-to-execution change), network picker (Mainnet, Hoodi, Gnosis, Chiado), online detector |
| Build | webpack x2, `bundle_proxy_*.sh/bat`, PyInstaller spec, electron-builder | Produces AppImage, dmg (x64 + arm64), portable exe |
| CI | `.github/workflows/build.yml` | 4-runner matrix, sha256 sidecars, build attestation, draft release with generated table |

Behavioural details worth preserving (or consciously changing):

- Mnemonic creation is always English (`createMnemonic('english')`).
- Mnemonic import today only validates **English**: the proxy calls
  `reconstruct_mnemonic(..., language='english')`. Abbreviated 4-letter words
  are accepted and expanded. We will accept all 10 BIP-39 languages (extra).
- Address validation in the UI uses `web3-utils.isAddress` (all-lower or
  all-upper accepted; mixed case must satisfy EIP-55). The proxy only checks
  hex form and lowercases. Rust will implement the `isAddress` semantics.
- The UI converts the ETH amount to gwei with float math
  (`parseInt((amount * multiplier * 1e9).toString())`). The Rust port should
  receive the amount as a decimal string and convert exactly (port of
  `validate_deposit_amount`).
- Files are created with `O_EXCL` and mode `0o400` on POSIX.
- File names: `keystore-m_12381_3600_{i}_0_0-{unix_secs}.json`,
  `deposit_data-{unix_secs}.json`, `bls_to_execution_change-{unix_secs}.json`.
- After writing, everything is re-read and verified (keystore decrypts to the
  same secret, deposit signature and roots verify, BTEC signature verifies).
- Error text `"That is not a valid mnemonic"` is string-matched by
  `MnemonicImport.tsx` (`MNEMONIC_ERROR_SEARCH`).

## 3. Target architecture

```
wagyu-key-gen/
├── Cargo.toml                    # workspace: crates/wagyu-core, crates/wagyu-cli, src-tauri
├── rust-toolchain.toml           # stable (Lighthouse v8.2.2 uses edition 2024 -> Rust >= 1.85)
├── crates/
│   ├── wagyu-core/               # pure Rust library, no Tauri dependency, all domain logic + tests
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── error.rs          # thiserror enum; Display text is what the UI shows
│   │   │   ├── chain.rs          # ChainSetting table (port of settings.py)
│   │   │   ├── mnemonic.rs       # create, reconstruct (abbreviation expansion, language detection), seed
│   │   │   ├── ssz.rs            # DepositMessage, DepositData, BLSToExecutionChange, Signed…, SigningData, ForkData, domains
│   │   │   ├── credential.rs     # Credential: keys for one validator index, withdrawal creds, deposit datum, BTEC
│   │   │   ├── keystore.rs       # thin wrapper over eth2_keystore (scrypt default, pbkdf2 option), file naming
│   │   │   ├── validation.rs     # address, amount, index/credential list parsing, password length, deposit/BTEC verification
│   │   │   ├── fs.rs             # sensitive file creation, dir checks, find_first_file
│   │   │   ├── deposit.rs        # generate_keys() orchestration
│   │   │   └── btec.rs           # validate_bls_credentials(), generate_bls_to_execution_change()
│   │   └── tests/
│   │       ├── vectors/          # JSON vectors copied from the vendored deposit-cli + golden files (section 7)
│   │       ├── mnemonic.rs  key_derivation.rs  keystore.rs  ssz.rs  validation.rs
│   │       ├── generate_keys.rs  btec.rs  regeneration.rs
│   │       └── golden.rs         # parity against Python-generated fixtures
│   └── wagyu-cli/                # optional: clap binary mirroring the 5 proxy subcommands (dev/QA/scripting)
├── src-tauri/
│   ├── Cargo.toml  build.rs  tauri.conf.json
│   ├── capabilities/default.json
│   ├── icons/                    # generated by `tauri icon static/icon.png`
│   └── src/{main.rs, lib.rs, commands.rs}
├── src/                          # React app (moved from src/react), pages/components unchanged
│   ├── api/index.ts              # NEW bridge: invoke + plugins, same method names as before
│   └── …
├── index.html  vite.config.ts  package.json  tsconfig.json  tailwind.config.js  postcss.config.js
├── static/icon.png               # icon source (see note in section 9)
└── .github/workflows/{build.yml, ci.yml}
```

Deleted at the end: `src/electron/`, `src/scripts/`, `src/vendors/`,
`webpack.*.config.js`, `build/icon.icns`, all Python/PyInstaller/Electron
references in `package.json` and `README.md`.

## 4. Dependencies (pinned)

Lighthouse's crypto crates are **not published on crates.io** (the crates.io
`bls` is an unrelated 0.0.0 placeholder). They must be git dependencies pinned
to a tag; latest tag today is `v8.2.2`.

`crates/wagyu-core/Cargo.toml`:

```toml
[dependencies]
# Lighthouse (all crypto). Pin tag AND rev for reproducibility.
bls                 = { git = "https://github.com/sigp/lighthouse", tag = "v8.2.2", default-features = false, features = ["supranational"] }
eth2_keystore       = { git = "https://github.com/sigp/lighthouse", tag = "v8.2.2" }
eth2_key_derivation = { git = "https://github.com/sigp/lighthouse", tag = "v8.2.2" }
eth2_wallet         = { git = "https://github.com/sigp/lighthouse", tag = "v8.2.2" }

# BIP-39 (all 10 languages). eth2_wallet re-exports tiny-bip39, which lacks Czech and Portuguese.
bip39 = { version = "2.2", features = ["all-languages"] }

# SSZ / Merkleization. MUST match the versions Lighthouse v8.2.2 pins so Hash256/TreeHash types line up.
tree_hash        = "0.12"
tree_hash_derive = "0.12"
ethereum_ssz        = "0.10"
ethereum_ssz_derive = "0.10"
alloy-primitives = { version = "1", default-features = false }   # Address (EIP-55), B256

serde = { version = "1", features = ["derive"] }
serde_json = "1"
hex = "0.4"
rand = "0.9"           # OsRng for mnemonic entropy
zeroize = { version = "1", features = ["zeroize_derive"] }
thiserror = "2"
rayon = "1"            # parallel keystore encryption
unicode-normalization = "0.1"   # NFKC for 4-letter abbreviation matching (same as deposit-cli)
tempfile = "3"         # is_directory_writable

[features]
portable = ["bls/supranational-portable"]   # x86_64 release builds: no ADX requirement (see risks)

[dev-dependencies]
tempfile = "3"
```

`src-tauri/Cargo.toml`: `tauri = "2"`, `tauri-build = "2"` (build-dep),
`tauri-plugin-dialog = "2"`, `tauri-plugin-clipboard-manager = "2"`,
`tauri-plugin-opener = "2"`, `serde`, `serde_json`, `wagyu-core`.
Feature `portable = ["wagyu-core/portable"]`.

Frontend `package.json`: add `@tauri-apps/api@^2`, `@tauri-apps/plugin-dialog`,
`@tauri-apps/plugin-clipboard-manager`, `@tauri-apps/plugin-opener`; dev:
`@tauri-apps/cli@^2`, `vite`, `@vitejs/plugin-react`, `vitest`,
`@testing-library/react`. Remove `electron`, `electron-builder`, `webpack*`,
`ts-loader`, `*-loader`, `html-webpack-plugin`, `git-revision-webpack-plugin`,
`web3-utils`, `@types/tmp`. Keep yarn 4, React 17, MUI 5, react-router 5 for
the migration (bumping React is a separate, later change).

Where each cryptographic operation comes from:

| Operation | Source |
|---|---|
| Entropy for a new mnemonic | `rand::rngs::OsRng` (32 bytes) → `bip39::Mnemonic::from_entropy_in` |
| Mnemonic → seed (PBKDF2-HMAC-SHA512, 2048 rounds) | `bip39::Mnemonic::to_seed_normalized(passphrase)` |
| Seed → master SK → child SKs (EIP-2333) | `eth2_wallet::recover_validator_secret_from_mnemonic(seed, index, KeyType::{Withdrawal, Voting})` which folds `eth2_key_derivation::DerivedKey::from_seed(..).child(..)` over `ValidatorPath` |
| Secret/public keys, sign, verify (BLS12-381, blst) | `bls::{SecretKey, PublicKey, Signature, Keypair, PublicKeyBytes, SignatureBytes}` |
| BLS withdrawal credentials `0x00 ‖ sha256(pk)[1..]` | `bls::get_withdrawal_credentials(&pk, 0x00)` |
| EIP-2335 keystore encrypt/decrypt (scrypt n=2^18 r=8 p=1 or PBKDF2 c=2^18) | `eth2_keystore::{KeystoreBuilder, Keystore, default_kdf, json_keystore::Kdf}` |
| Password NFKD normalisation + control-char stripping | inside `eth2_keystore::{encrypt, decrypt}` |
| SSZ hash-tree-root (sha256) | `tree_hash` / `ethereum_hashing` |
| EIP-55 address checksum (keccak) | `alloy_primitives::Address::{parse_checksummed, to_checksum}` |

## 5. Behaviour mapping

### 5.1 Backend API (`wagyu-core`)

```rust
pub enum Network { Mainnet, Hoodi, Gnosis, Chiado, Sepolia, Holesky, Ephemery }
pub struct ChainSetting { name, genesis_fork_version: [u8; 4], exit_fork_version: [u8; 4],
                          genesis_validators_root: Option<Hash256>, multiplier: u64,
                          min_activation_amount_eth: u64 /* or exact decimal */, min_deposit_amount_gwei: u64 }

pub fn create_mnemonic(language: bip39::Language) -> Zeroizing<String>;
/// Accepts abbreviated words, any language, returns the full-word mnemonic.
pub fn reconstruct_mnemonic(input: &str, language: Option<bip39::Language>) -> Result<Zeroizing<String>, Error>;

pub struct Credential { /* index, signing/withdrawal SecretKey, chain, withdrawal_address: Option<Address>, compounding, amount_gwei */ }
impl Credential {
    pub fn new(seed: &[u8], index: u32, amount_gwei: u64, chain: &ChainSetting,
               withdrawal_address: Option<Address>, compounding: bool) -> Result<Self, Error>;
    pub fn withdrawal_credentials(&self) -> Hash256;      // 0x00 / 0x01 / 0x02 forms
    pub fn deposit_datum(&self) -> Result<DepositDatum, Error>;   // serde struct, field order as Python
    pub fn signing_keystore(&self, password: &[u8], kdf: KdfChoice) -> Result<Keystore, Error>;
    pub fn bls_to_execution_change(&self, validator_index: u64) -> Result<BtecEntry, Error>;
}

pub struct GenerateKeysRequest { mnemonic, mnemonic_password: String /* "" */, start_index: u32, count: u32,
    amount_eth: String /* decimal, exact */, network: Network, keystore_password: Zeroizing<String>,
    withdrawal_address: Option<String>, compounding: bool, folder: PathBuf, kdf: KdfChoice /* Scrypt | Pbkdf2 */ }
pub struct GenerateKeysOutput { keystore_files: Vec<PathBuf>, deposit_data_file: PathBuf }
pub fn generate_keys(req: &GenerateKeysRequest) -> Result<GenerateKeysOutput, Error>;

pub fn validate_bls_credentials(network, mnemonic, start_index, creds: &[Hash256]) -> Result<(), Error>;
pub fn generate_bls_to_execution_change(folder, network, mnemonic, start_index,
        validator_indices: &[u64], creds: &[Hash256], withdrawal_address: &str) -> Result<PathBuf, Error>;
```

`generate_keys` steps (mirrors `stakingdeposit_proxy.generate_keys`):
1. `reconstruct_mnemonic` → seed.
2. Parse/normalise withdrawal address (`isAddress` semantics), chain setting,
   amount string → gwei with bounds `min_deposit_amount * multiplier * 1e9 ≤ amount ≤ 2048e9`.
3. Create folder if missing; `timestamp = unix seconds`.
4. Build `count` credentials for indices `start..start+count` (rayon).
5. Encrypt keystores in parallel with a **bounded** pool (scrypt n=2^18, r=8
   uses ~256 MiB per concurrent job; cap at `min(available_parallelism, 4)`),
   write with `O_EXCL | 0o400`.
6. Build deposit data list, write `deposit_data-{ts}.json`.
7. Verify: decrypt every keystore and compare the secret; run
   `validate_deposit` (pubkey, credentials form, amount bounds, signature,
   `deposit_data_root`) on the re-read JSON. Any failure → error (files are
   left in place, as today).

SSZ containers mirror Lighthouse `types` field types exactly so the
`tree_hash`/`bls` impls are reused: `DepositMessage { pubkey: PublicKeyBytes,
withdrawal_credentials: Hash256, amount: u64 }`, `DepositData { …, signature:
SignatureBytes }`, `BlsToExecutionChange { validator_index: u64,
from_bls_pubkey: PublicKeyBytes, to_execution_address: Address }`,
`SignedBlsToExecutionChange { message, signature: SignatureBytes }`,
`SigningData { object_root: Hash256, domain: Hash256 }`, `ForkData {
current_version: [u8; 4], genesis_validators_root: Hash256 }`. Domains:
`DOMAIN_DEPOSIT = 0x03000000` (genesis validators root = zero),
`DOMAIN_BLS_TO_EXECUTION_CHANGE = 0x0A000000`, `DOMAIN_VOLUNTARY_EXIT =
0x04000000` (only if exit transactions are built).

### 5.2 Tauri commands (`src-tauri/src/commands.rs`)

| Old bridge call | Tauri replacement |
|---|---|
| `eth2Deposit.createMnemonic(language)` | `invoke('create_mnemonic', { language })` → `String` |
| `eth2Deposit.validateMnemonic(m)` | `invoke('validate_mnemonic', { mnemonic })` → returns the **reconstructed** mnemonic; UI stores that (today it stores the cleaned input; Python re-reconstructs later anyway) |
| `eth2Deposit.generateKeys(…)` | `invoke('generate_keys', { request })` (async command, runs on a blocking thread; optional `keygen-progress` events for the loader) |
| `eth2Deposit.validateBLSCredentials(…)` | `invoke('validate_bls_credentials', …)` |
| `eth2Deposit.generateBLSChange(…)` | `invoke('generate_bls_change', …)` → file path |
| `bashUtils.doesDirectoryExist / isDirectoryWritable / findFirstFile` | same names as commands, implemented in `wagyu-core::fs` |
| `web3Utils.isAddress` | `invoke('is_address', { address })` (alloy) |
| `electronAPI.invokeShowOpenDialog({openDirectory})` | `@tauri-apps/plugin-dialog` `open({ directory: true, multiple: false })` |
| `electronAPI.clipboardWriteText` | `@tauri-apps/plugin-clipboard-manager` `writeText` |
| clear clipboard on quit | Rust: `RunEvent::Exit` → `app.clipboard().clear()` |
| `electronAPI.shellShowItemInFolder` | `@tauri-apps/plugin-opener` `revealItemInDir` |
| `electronAPI.ipcRendererSendClose` | `invoke('quit')` → `app.exit(0)` (goes through the Exit hook above) |
| `VERSION` / `COMMITHASH` webpack defines | `invoke('version_info')` → `{ version, commit }`; `build.rs` exports `WAGYU_COMMIT` via `git rev-list --max-count=1 --no-merges --abbrev-commit HEAD`; window title set in `setup` |

Error contract: commands return `Result<T, String>`; the frontend replaces
`('stderr' in error) ? error.stderr : error.message` with `String(error)`.
The invalid-mnemonic message keeps the text
`"That is not a valid mnemonic, please check for typos."` so the existing
string match keeps working, or (better) `MnemonicImport.tsx` switches to a
typed error code returned by the command.

### 5.3 Tauri configuration

- `tauri.conf.json`: `productName "Wagyu Key Gen"`, `identifier
  "gg.wagyu.keygen"`, `version: "../package.json"` (single source of truth),
  window 950x750 with the title above, `build.beforeDevCommand "yarn dev"`,
  `beforeBuildCommand "yarn build"`, `frontendDist "../dist"`, `devUrl
  http://localhost:1420`.
- `bundle.targets: ["appimage", "dmg", "nsis"]`,
  `bundle.windows.webviewInstallMode: { "type": "downloadBootstrapper" }`.
  Also publish the bare `target/release/wagyu-key-gen.exe` as the "portable"
  artifact (runs wherever WebView2 is already installed, which is every
  Windows 11 and updated Windows 10).
- Security: `app.security.csp` with `default-src 'self'; style-src 'self'
  'unsafe-inline'` (MUI/emotion injects styles at runtime; Tauri appends what
  its IPC needs). `capabilities/default.json` grants only: `core:default`,
  `dialog:allow-open`, `clipboard-manager:allow-write-text`,
  `opener:allow-reveal-item-in-dir`, plus the app's own commands. Devtools are
  off in release builds (Tauri default). No `shell` plugin.
- macOS gets a minimal app menu (Quit, Copy/Paste) so keyboard shortcuts work;
  Windows/Linux have no menu bar, matching today.
- Online detector keeps using `navigator.onLine`; verify on WebKitGTK, which
  is known to be less reliable than Chromium for this signal.

## 6. Frontend migration (keep React, swap the bridge)

1. Move `src/react/**` to `src/`, `src/react/index.html` to the repo root
   (Vite convention), keep Tailwind `tw-` prefix and `theme.ts` untouched.
2. Create `src/api/index.ts` exporting `eth2Deposit`, `electronAPI`,
   `bashUtils`, `web3Utils` objects with the **same method names and
   signatures** as `renderer.d.ts`, implemented with `invoke` and the plugins.
   Pages then only change `window.eth2Deposit.` → `eth2Deposit.` (import),
   plus the error handling line. Delete `renderer.d.ts` and the global
   `Window` augmentation.
3. `FolderSelector.tsx`: drop the `electron` `OpenDialogOptions` import; call
   the bridge's `invokeShowOpenDialog()` which returns `string | null`.
4. `VersionFooter.tsx`: read `{version, commit}` from `version_info` (a tiny
   hook with `useEffect`) instead of webpack defines.
5. `CreateValidatorKeys.tsx`: send `amount` as the decimal string the user
   typed (plus `network`) and let Rust do the gwei conversion; drop
   `ETH_TO_GWEI` float math.
6. `MnemonicImport.tsx`: store the reconstructed mnemonic returned by
   `validate_mnemonic`; keep the length pre-check.
7. Vite: `vite.config.ts` with `@vitejs/plugin-react`, `server.port 1420
   strictPort`, `envPrefix ['VITE_', 'TAURI_ENV_*']`, `build.target`
   per Tauri docs (`es2021`/`safari13`), `clearScreen: false`.
8. Scripts: `dev: vite`, `build: tsc && vite build`, `tauri: tauri`,
   `test: vitest`. Local run: `yarn tauri dev`. Release: `yarn tauri build`.
9. Vitest: `helpers.test.ts` (`cleanMnemonic`), and wizard smoke tests with
   the bridge mocked via `vi.mock('../api')` or `@tauri-apps/api/mocks`
   (`mockIPC`): mnemonic length error, password mismatch, BTEC input
   validation. These are new (the Electron app has no tests).

## 7. Golden fixtures (do this before deleting the Python code)

The Python CLI tests do not contain hard-coded pubkeys or signatures; they
only check credential prefixes, amounts, file counts and permissions. To get a
real parity check, generate fixtures with the **existing** Python proxy and
commit them under `crates/wagyu-core/tests/vectors/golden/`:

1. `python3 -m venv .venv && pip install -r src/vendors/ethstaker-deposit-cli-1.2.2/requirements.txt`
   (requires Python ≥ 3.9; the machine has 3.9.6).
2. Run `src/scripts/stakingdeposit_proxy.py generate_keys` for a fixed matrix:
   - mnemonic `abandon … about` (EIP-2333 vector), index 0, count 3, mainnet,
     32 ETH, no address (0x00 credentials);
   - same with address `0x00000000219ab540356cBB839Cbe05303d7705Fa` (0x01);
   - same with `--compounding` and amounts 32, 1050, `1.000000001` (0x02);
   - Hoodi 32 ETH; Gnosis and Chiado 1 GNO (→ 32e9 gwei) and 2.5 GNO.
3. Run `bls_change` with the deposit-cli test inputs: mnemonic `sister protect
   peanut hill … paper worry`, credentials `0x00bd0b5a…6aa8de` and
   `0x00a75d83…d5382`, start index 0, validator indices `1,2`, address
   `0x3434…3434`, mainnet; repeat for Hoodi/Gnosis/Chiado.
4. Keep the `deposit_data-*.json` and `bls_to_execution_change-*.json` files
   verbatim (BLS signatures are deterministic, so these are byte-comparable
   after JSON parsing). Keep one keystore per KDF for decrypt tests (salt/iv/uuid
   are random, so compare decrypted secret + pubkey + path only).
5. `tests/golden.rs` re-generates each case in Rust and asserts equality of
   the parsed JSON (deposit data, BTEC) and of pubkey/path/decrypted secret
   (keystores).

Also copy the vendored vectors as-is: `tests/test_key_handling/keystore_test_vectors/test{0,1}.json`,
`test_key_derivation/test_vectors/{mnemonic,multi_lang_mnemonic,tree_kdf,tree_kdf_intermediate}.json`.

## 8. Test porting matrix

Legend: **PORT** = same assertions in Rust; **ADAPT** = same intent via the
public API; **DROP** = not applicable (reason given).

| Python test | Decision | Rust location / reason |
|---|---|---|
| `test_mnemonic.py::test_bip39` (9 languages, entropy→mnemonic, mnemonic+"TREZOR"→seed) | PORT | `tests/mnemonic.rs` via `Mnemonic::from_entropy_in` + `to_seed_normalized` |
| `::test_reconstruct_mnemonic` | PORT | `reconstruct_mnemonic` returns the full mnemonic for every vector |
| `::test_multi_lang_mnemonics` | PORT | expect `Error::AmbiguousLanguages` for `multi_lang_mnemonic.json` |
| `::test_reconstruct_abbreviated_mnemonic` | PORT | abbreviate every word to 4 NFKC chars, reconstruct equals full mnemonic |
| `::test_get_word` (index 2048) | DROP | word lists are `[&str; 2048]`, unrepresentable |
| `::test_determine_mnemonic_language(_error)` | PORT | `detect_languages` set equality; unknown words → error |
| `test_tree.py::test_hkdf_mod_r`, `::test_hkdf_mod_r_key_info` | DROP | tests Python HKDF internals; covered by Lighthouse `eth2_key_derivation/tests/eip2333_vectors.rs` |
| `::test_derive_master_SK` | PORT | `DerivedKey::from_seed(seed).secret() == master_SK` for `tree_kdf.json`; seed < 32 bytes → `Err` |
| `::test_derive_child_SK_valid` | PORT (valid half) | `.child(index)`; the `2**32` case is unrepresentable with `u32` |
| `test_path.py::test_flip_bits_256`, `::test_IKM_to_lamport_SK`, `::test_parent_SK_to_lamport_PK`, `::test_HKDF_mod_r` | DROP | Lamport intermediates are private to the implementation; covered upstream |
| `::test_mnemonic_and_path_to_key` | PORT | `abandon…about` + `TREZOR` → seed → `m/0` child == `child_SK` |
| `::test_path_to_nodes` | ADAPT | assert `ValidatorPath::new(i, Voting).to_string() == "m/12381/3600/i/0/0"`; invalid-path cases dropped (no free-form path parser) |
| `test_keystore.py::test_json_serialization` | PORT | `Keystore::from_json_str` → `to_json_string` → parsed JSON equal to vector |
| `::test_encrypt_decrypt_test_vectors` | ADAPT (decrypt) | decrypt `test0` (pbkdf2) and `test1` (scrypt) with `𝔱𝔢𝔰𝔱𝔭𝔞𝔰𝔰𝔴𝔬𝔯𝔡🔑` → expected secret; exercises NFKD |
| `::test_generated_keystores` (byte-identical crypto with fixed iv) | DROP | Lighthouse's builder chooses the IV; reproducing it would mean writing cipher code. Covered by `eth2_keystore/tests/eip2335_vectors.rs` |
| `::test_encrypt_decrypt_{pbkdf2,scrypt}_random_iv` | PORT | round-trip with `default_kdf` and `Kdf::Pbkdf2` |
| `::test_encrypt_decrypt_incorrect_password` | PORT | expect `Err` |
| `::test_process_password` | ADAPT | encrypt with `"a\u{8}c"`, decrypt with `"ac"` succeeds (control chars stripped) |
| `test_crypto.py::*` (scrypt/PBKDF2/AES parameter guards of Python wrappers) | DROP | no wrappers of our own; Lighthouse `eth2_keystore/tests/params.rs` covers KDF parameter validation |
| `test_ssz.py::test_compute_deposit_domain`, `::test_compute_deposit_fork_data_root`, `::test_compute_signing_root` | PORT | `tests/ssz.rs`, exact expected bytes; wrong-length cases dropped (fixed-size types) |
| `test_constants.py::test_add_index_to_options` | DROP | CLI menu numbering |
| `test_validation.py::test_validate_password_strength` | PORT | `validate_password` (≥ 12 chars), also enforced in Rust as defence in depth |
| `::test_validate_password_strength_encoding` | DROP | stdin encoding is meaningless in a GUI |
| `::test_validate_int_range` | PORT | used by validator-index parsing (`0 ≤ i < 2^32`) |
| `::test_validate_deposit_amount` | PORT (mainnet/gnosis/chiado rows) | exact decimal → gwei; `devnet` rows dropped (no custom chains in Wagyu) |
| `::test_normalize_input_list` | PORT | parsing of comma/semicolon/space separated indices and credentials |
| `::test_validate_signed_exit` | DROP now, PORT in phase 7 if exit transactions are built | |
| `::test_validate_devnet_chain_setting_json` | DROP | no devnet UI |
| `test_credentials.py::test_from_mnemonic` (amounts/keys mismatch) | DROP | API takes `count` + one amount; add `count == 0 → Err` instead |
| `test_deposit.py::*` (python version, connectivity prompts) | DROP | not applicable; UI has `OnlineDetector` |
| `test_intl/*` | DROP | no i18n layer |
| `test_cli/test_new_mnemonic.py`, `test_existing_mnemonic.py` (bls withdrawal, withdrawal address, compounding, gnosis/chiado, custom amount, decimal amount, bad checksum, TREZOR passphrase, pbkdf2 vs scrypt equality, multiple languages) | ADAPT | `tests/generate_keys.rs` over `wagyu_core::generate_keys` in a temp dir: file set, unique uuids, `0o400` perms on Unix, credential prefix/address, amount, deposit verification, plus golden parity. `custom_testnet` and `test_script*` rows dropped |
| `test_cli/test_regeneration.py` | PORT | indices 0–1 then 1–2 in another dir; index-1 keystore has the same pubkey and path |
| `test_cli/test_generate_bls_to_execution_change.py` (non-interactive, multiple) | PORT + golden content check | `tests/btec.rs`; `custom_testnet` and interactive variants dropped |
| `test_cli/test_generate_bls_to_execution_change_keystore.py`, `test_exit_transaction_*.py`, `test_partial_deposit.py`, `test_test_keystore.py` | DROP | features Wagyu does not expose (exit transactions revisited in phase 7) |
| `test_binary_*.py`, `test_*_script.py` (drive the PyInstaller binary) | DROP | replaced by an optional `wagyu-cli` smoke test in CI and a manual Tauri QA checklist |

## 9. Phases and deliverables

Estimates are rough working days for one engineer familiar with Rust.

**Phase 0 — Scaffold and fixtures (0.5 d)**
- Cargo workspace, `rust-toolchain.toml`, `crates/wagyu-core` skeleton, copy
  vectors, generate golden fixtures (section 7) while Python still exists.
- Check the Lighthouse git dependency builds on this machine (blst compiles C;
  Xcode CLT is present).

**Phase 1 — `wagyu-core` domain modules with unit tests (2–3 d)**
- `chain.rs`, `mnemonic.rs`, `ssz.rs`, `credential.rs`, `keystore.rs`,
  `validation.rs`, `fs.rs`; port the unit-level tests from section 8.
- Acceptance: EIP-2333/2335 vectors, BIP-39 vectors for 9 languages,
  abbreviation/ambiguity behaviour, SSZ expected bytes all green.

**Phase 2 — Orchestration + integration tests (1–2 d)**
- `deposit.rs`, `btec.rs`; `tests/generate_keys.rs`, `btec.rs`,
  `regeneration.rs`, `golden.rs`.
- Optional `wagyu-cli` (clap) exposing the 5 subcommands for manual QA.
- Acceptance: golden parity passes; 100-key generation on a laptop finishes in
  a few minutes with bounded memory.

**Phase 3 — Tauri shell (1 d)**
- `src-tauri` with commands, plugins, capabilities, CSP, exit hook clearing
  the clipboard, window/title/menu, `build.rs` commit hash, icons via
  `yarn tauri icon static/icon.png` (the source should be ≥ 1024×1024; the
  current `static/icon.png` is smaller, so export a larger PNG from the Figma
  design first).

**Phase 4 — Frontend migration (1–2 d)**
- Section 6. Acceptance: all three wizard flows work end-to-end with
  `yarn tauri dev` on macOS, producing files identical in shape to today's.

**Phase 5 — CI/CD (1 d)**
- New `ci.yml` on PRs: `cargo fmt --check`, `cargo clippy --workspace
  -D warnings`, `cargo test --workspace`, `yarn test`, `yarn build`.
- Rewrite `build.yml`: same runner matrix (`ubuntu-22.04`, `macos-15-intel`,
  `macos-latest`, `windows-latest`); install Rust (`dtolnay/rust-toolchain`),
  Node + corepack yarn, Linux packages (`libwebkit2gtk-4.1-dev`,
  `libappindicator3-dev`, `librsvg2-dev`, `patchelf`); `yarn tauri build`
  (`--features portable` on x86_64 runners); pick artifacts from
  `src-tauri/target/release/bundle/{appimage,dmg,nsis}` plus the bare Windows
  exe; keep the SHORT_SHA rename, sha256 sidecars, attestation and draft
  release. Update the platform detection in the release script for Tauri's
  file names (`…_aarch64.dmg`, `…_x64.dmg`, `…_amd64.AppImage`,
  `…_x64-setup.exe`).
- Update `.github/release_template.md`: keep the AppImage/FUSE notes, drop the
  Electron SUID-sandbox/AppArmor section (re-verify WebKitGTK's bubblewrap
  sandbox on Ubuntu 24.04 and keep a softened note if needed), add a line that
  Windows needs the WebView2 runtime (bootstrapper downloads it if absent).

**Phase 6 — Cleanup and docs (0.5 d)**
- Delete Electron/Python/webpack files (section 3), rewrite the README build
  sections (Rust toolchain, Tauri prerequisites per OS, `yarn tauri dev/build`),
  keep the GPL-3 license, add a note that the HashCloak audit covered the
  Electron/Python implementation and does not cover this rewrite.
- Bump version to 2.0.0 in `package.json` (Tauri reads it; Cargo crate
  versions follow).

**Phase 7 — Low-cost extras (optional, 1–3 d total, each independent)**
- PBKDF2 keystore toggle in `ConfigureValidatorKeys` (core already supports it).
- Any-language import is already included by default (no UI change). A
  language picker for *creating* mnemonics was considered and rejected:
  Wagyu, Lighthouse, Nimbus and ethdo all create English-only mnemonics, and
  the picker would need real UI work (24-cell grid with CJK words) for little
  benefit. The core keeps a `language` parameter so it can be added later.
- Exit transaction generation (needs `VoluntaryExit` SSZ, exit fork versions
  already in `ChainSetting`, a new wizard flow); port
  `test_validate_signed_exit` and the exit-transaction CLI tests then.
- Progress events during key generation to replace the static loader text.

## 10. Risks, decisions and assumptions

- **Lighthouse via git**: cargo clones the whole Lighthouse repo on first
  build (large) and blst needs a C compiler on every platform. Mitigation: pin
  `tag` + `rev`, cache `~/.cargo` in CI, document the toolchain requirements.
- **CPU compatibility**: default `blst` builds may use ADX instructions and
  crash with an illegal instruction on pre-2015 x86 CPUs, which is why
  Lighthouse ships separate "portable" binaries. Wagyu targets arbitrary user
  hardware, so x86_64 release builds use the `portable` feature; arm64 builds
  are unaffected.
- **Rust version**: Lighthouse v8.2.2 uses edition 2024, so Rust ≥ 1.85.
- **`deposit_cli_version` field**: assumption is to keep the constant
  `"1.2.2"` (the format version we reproduce) so the launchpad's version check
  keeps passing. Verify against the launchpad's `validateDepositKey` source
  before the first release; switching to Wagyu's own semver is a one-line
  change.
- **JSON field order** differs from Python for keystores (Lighthouse writes
  `crypto, uuid, path, pubkey, version, description` order). Consumers parse
  objects, so this is harmless; golden tests compare parsed JSON.
- **Memory during key generation**: scrypt at n=2^18 uses ~256 MiB per
  concurrent keystore. The Python version used one process per core; the Rust
  version bounds the pool so 1000 keys cannot exhaust RAM.
- **Secrets in memory**: mnemonic, seed and secret keys are `Zeroizing` /
  `PlainText` in Rust; strings crossing the IPC into JS cannot be zeroed, same
  as today with Electron.
- **Audit**: this is a rewrite of audited code. The golden-fixture parity suite
  is the main safety net; recommend a fresh review of `wagyu-core` before
  promoting a mainnet release.
- **Behaviour changes** (all deliberate, listed for the changelog): import
  accepts all BIP-39 languages; exact decimal amount conversion; reconstructed
  mnemonic is what gets used downstream; Windows artifact becomes an NSIS
  installer plus a bare exe instead of the Electron "portable" exe.
- **Repo layout** assumed: monorepo with `crates/` + `src-tauri/` + `src/`;
  yarn 4 stays as the JS package manager.

## 11. Definition of done

- `cargo test --workspace` green, including golden parity.
- `yarn test` green; `yarn tauri dev` runs all three flows on macOS, Linux
  and Windows; files open in the launchpad (deposit_data) and beaconcha.in
  broadcast tool (BTEC) without complaint.
- `build.yml` produces AppImage, x64 + aarch64 dmg, NSIS exe + bare exe with
  sha256 sidecars, attestation and the draft release table.
- No Python, PyInstaller, Electron or webpack left in the repository.

## 12. Implementation notes (deviations from the plan)

- The SSZ module is called `spec.rs`, not `ssz.rs`, to avoid clashing with the `ssz` crate name
  inside derive-macro expansions. The `ethereum_ssz` dependency turned out to be unnecessary:
  only `tree_hash` is needed for signing roots.
- Keystore JSON is written through a small `KeystoreFile` struct so the field set and order
  match the deposit-cli exactly (Lighthouse would add a `"name": null` field). Reading accepts
  both layouts.
- `deposit_amount_to_gwei` lives in the core and is called from the Tauri command layer, so the
  core API takes gwei like the Python proxy did while the UI sends the exact decimal string.
- The Tauri IPC layer is tested through `tauri::test` (mock runtime, real generated context) in
  `src-tauri/tests/ipc.rs`. Requests must use the dev URL as origin, otherwise Tauri treats them
  as remote and refuses application commands.
- `scrypt` and friends get `opt-level = 3` in the dev profile (workspace `Cargo.toml`), otherwise
  the test suite takes minutes instead of seconds.
- Yarn was upgraded to 4.18 (4.1 fails its link step on Node 24) and Vite to 7 so that Vitest
  and Vite share one Vite version.
- Golden fixtures: 15 cases generated from the Python proxy before it was deleted, under
  `crates/wagyu-core/tests/vectors/golden`. Deposit data and BTEC files compare equal as parsed
  JSON; keystores compare by pubkey, path and decrypted secret.
- CI: `ci.yml` (fmt, clippy, tests on three OSes, frontend tests and build) is new; `build.yml`
  now runs the Tauri CLI and ships the NSIS installer plus the bare exe for Windows.
- Package manager: switched from Yarn 4 (the project's prior choice) to npm. The frontend used
  none of Yarn Berry's differentiating features here (no workspaces, `nodeLinker: node-modules`
  so no Plug'n'Play, no plugins), so it added config and a vendored binary
  (`.yarn/releases/*.cjs`) for no benefit over npm, which every contributor already has. `yarn
  <cmd>` becomes `npm run <cmd>`; passing flags through the `tauri` script needs `--`, e.g.
  `npm run tauri build -- --features portable`.
- Clipboard clear on quit fires on `RunEvent::ExitRequested`, not `RunEvent::Exit` as section 5.2
  originally said. `tauri-plugin-clipboard-manager` registers its own `RunEvent::Exit` handler
  that takes and drops its `arboard::Clipboard` (arboard requires that drop to flush a write to
  the OS clipboard), and Tauri runs a plugin's `on_event` before the app's own `.run` callback for
  the same event — so clearing at `Exit` found the clipboard already taken and panicked silently,
  clearing nothing. No automated test: `arboard::Clipboard::new()` needs a real OS clipboard and
  display session, which GitHub's Linux CI runner doesn't have, so a test exercising this would be
  flaky on exactly the platform most likely to hide the regression again. Verify by hand.
