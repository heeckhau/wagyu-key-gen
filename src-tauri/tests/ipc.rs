//! Exercises the IPC command layer through Tauri's mock runtime: the same JSON the TypeScript
//! bridge (`src/api/index.ts`) sends, through the real command handlers, down to `wagyu-core`.

use serde_json::{json, Value};
use tauri::ipc::{CallbackFn, InvokeBody, InvokeResponseBody};
use tauri::test::{get_ipc_response, mock_builder, MockRuntime, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::{App, WebviewWindow};

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const SISTER: &str = "sister protect peanut hill ready work profit fit wish want small inflict flip member tail between sick setup bright duck morning sell paper worry";
const CREDS: &str = "0x00bd0b5a34de5fb17df08410b5e615dda87caf4fb72d0aac91ce5e52fc6aa8de,0x00a75d83f169fa6923f3dd78386d9608fab710d8f7fcf71ba9985893675d5382";
const MAINNET_KEYSTORE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../crates/wagyu-core/tests/vectors/golden/keys_mainnet_bls_3/keystore-0.json"
);
const MAINNET_PUBKEY: &str = "b3e445d43871965d890a398f719348a1405ac72e35b92727cc570026f54471af7ea7b2040622a8fd0b5bfb2a209b5911";

fn app() -> (App<MockRuntime>, WebviewWindow<MockRuntime>) {
    let app = wagyu_key_gen_lib::register_commands(mock_builder())
        .build(tauri::generate_context!())
        .expect("mock app");
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("mock webview");
    (app, webview)
}

fn invoke(webview: &WebviewWindow<MockRuntime>, cmd: &str, args: Value) -> Result<Value, Value> {
    get_ipc_response(
        webview,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            // Must be "local" (relative to devUrl in dev/test builds) for app commands to be allowed.
            url: "http://localhost:1420/".parse().unwrap(),
            body: InvokeBody::Json(args),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|body| match body {
        InvokeResponseBody::Json(text) => serde_json::from_str(&text).expect("json response"),
        InvokeResponseBody::Raw(_) => panic!("unexpected raw response"),
    })
}

#[test]
fn small_commands() {
    let (_app, webview) = app();

    assert_eq!(
        invoke(
            &webview,
            "is_address",
            json!({"address": "0x00000000219ab540356cBB839Cbe05303d7705Fa"})
        ),
        Ok(json!(true))
    );
    assert_eq!(
        invoke(&webview, "is_address", json!({"address": "0x1234"})),
        Ok(json!(false))
    );

    assert_eq!(
        invoke(
            &webview,
            "validate_mnemonic",
            json!({"mnemonic": "aban aban aban aban aban aban aban aban aban aban aban abou"})
        ),
        Ok(json!(ABANDON))
    );
    assert_eq!(
        invoke(
            &webview,
            "validate_mnemonic",
            json!({"mnemonic": "abandon abandon"})
        ),
        Err(json!(
            "That is not a valid mnemonic, please check for typos."
        ))
    );
    assert_eq!(
        invoke(
            &webview,
            "validate_mnemonic",
            json!({"mnemonic": ABANDON, "language": "french"})
        ),
        Err(json!(
            "That is not a valid mnemonic, please check for typos."
        ))
    );

    let mnemonic = invoke(&webview, "create_mnemonic", json!({"language": "english"})).unwrap();
    assert_eq!(mnemonic.as_str().unwrap().split(' ').count(), 24);

    let info = invoke(&webview, "version_info", json!({})).unwrap();
    assert!(info["version"].is_string() && info["commit"].is_string());
}

#[test]
fn directory_commands() {
    let (_app, webview) = app();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_str().unwrap();
    std::fs::write(dir.path().join("keystore-m_12381_3600_0_0_0-1.json"), "{}").unwrap();

    assert_eq!(
        invoke(&webview, "does_directory_exist", json!({"directory": path})),
        Ok(json!(true))
    );
    assert_eq!(
        invoke(
            &webview,
            "is_directory_writable",
            json!({"directory": path})
        ),
        Ok(json!(true))
    );
    assert_eq!(
        invoke(
            &webview,
            "does_directory_exist",
            json!({"directory": format!("{path}/missing")})
        ),
        Ok(json!(false))
    );
    assert_eq!(
        invoke(
            &webview,
            "find_first_file",
            json!({"directory": path, "startsWith": "keystore"})
        ),
        Ok(json!(dir
            .path()
            .join("keystore-m_12381_3600_0_0_0-1.json")
            .to_str()
            .unwrap()))
    );
    assert_eq!(
        invoke(
            &webview,
            "find_first_file",
            json!({"directory": path, "startsWith": "deposit"})
        ),
        Ok(json!(""))
    );
}

#[test]
fn generate_keys_round_trip() {
    let (_app, webview) = app();
    let dir = tempfile::tempdir().unwrap();

    let result = invoke(
        &webview,
        "generate_keys",
        json!({"request": {
            "mnemonic": ABANDON,
            "index": 0,
            "amount": "1",
            "count": 1,
            "network": "Gnosis",
            "password": "MyPasswordIs",
            "withdrawalAddress": "0x00000000219ab540356cBB839Cbe05303d7705Fa",
            "compounding": true,
            "folder": dir.path().to_str().unwrap(),
        }}),
    )
    .unwrap();

    assert_eq!(result["keystore_files"].as_array().unwrap().len(), 1);
    assert_eq!(result["pubkeys"].as_array().unwrap().len(), 1);
    let deposit_file = result["deposit_data_file"].as_str().unwrap();
    let deposits: Value =
        serde_json::from_str(&std::fs::read_to_string(deposit_file).unwrap()).unwrap();
    // 1 GNO on Gnosis is a 32e9 gwei deposit with compounding (0x02) credentials.
    assert_eq!(deposits[0]["amount"], json!(32_000_000_000u64));
    assert_eq!(deposits[0]["network_name"], json!("gnosis"));
    assert!(deposits[0]["withdrawal_credentials"]
        .as_str()
        .unwrap()
        .starts_with("02"));

    // Errors come back as their display text.
    let error = invoke(
        &webview,
        "generate_keys",
        json!({"request": {
            "mnemonic": ABANDON, "index": 0, "amount": "0.5", "count": 1, "network": "Mainnet",
            "password": "MyPasswordIs", "withdrawalAddress": "", "compounding": false,
            "folder": dir.path().to_str().unwrap(),
        }}),
    )
    .unwrap_err();
    assert!(
        error
            .as_str()
            .unwrap()
            .contains("Amount must be at least 1"),
        "{error}"
    );
}

#[test]
fn bls_change_round_trip() {
    let (_app, webview) = app();
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(
        invoke(
            &webview,
            "validate_bls_credentials",
            json!({"chain": "Mainnet", "mnemonic": SISTER, "index": 0, "withdrawalCredentials": CREDS}),
        ),
        Ok(Value::Null)
    );
    let mismatch = invoke(
        &webview,
        "validate_bls_credentials",
        json!({"chain": "Mainnet", "mnemonic": SISTER, "index": 1, "withdrawalCredentials": CREDS}),
    )
    .unwrap_err();
    assert!(
        mismatch.as_str().unwrap().contains("do not match"),
        "{mismatch}"
    );

    let file = invoke(
        &webview,
        "generate_bls_change",
        json!({
            "folder": dir.path().to_str().unwrap(),
            "chain": "Mainnet",
            "mnemonic": SISTER,
            "index": 0,
            "indices": "1,2",
            "withdrawalCredentials": CREDS,
            "executionAddress": "0x3434343434343434343434343434343434343434",
        }),
    )
    .unwrap();
    let file = file.as_str().unwrap();
    assert!(file.contains("bls_to_execution_change-"));
    let entries: Value = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
    assert_eq!(entries.as_array().unwrap().len(), 2);
    assert_eq!(entries[1]["message"]["validator_index"], json!("2"));
}

#[test]
fn exit_transaction_round_trips() {
    let (_app, webview) = app();
    let dir = tempfile::tempdir().unwrap();

    let files = invoke(
        &webview,
        "generate_exit_transactions",
        json!({"request": {
            "folder": dir.path().to_str().unwrap(),
            "chain": "Mainnet",
            "mnemonic": SISTER,
            "index": 0,
            "indices": "1, 2",
            "epoch": 1234,
        }}),
    )
    .unwrap();
    let files = files.as_array().unwrap();
    assert_eq!(files.len(), 2);
    let second: Value =
        serde_json::from_str(&std::fs::read_to_string(files[1].as_str().unwrap()).unwrap())
            .unwrap();
    assert_eq!(
        second["message"],
        json!({"epoch": "1234", "validator_index": "2"})
    );

    let file = invoke(
        &webview,
        "generate_exit_transaction_keystore",
        json!({"request": {
            "folder": dir.path().to_str().unwrap(),
            "chain": "Hoodi",
            "keystore": MAINNET_KEYSTORE,
            "keystorePassword": "MyPasswordIs",
            "validatorIndex": 7,
        }}),
    )
    .unwrap();
    assert!(file
        .as_str()
        .unwrap()
        .contains("signed_exit_transaction-7-"));

    let wrong_password = invoke(
        &webview,
        "generate_exit_transaction_keystore",
        json!({"request": {
            "folder": dir.path().to_str().unwrap(),
            "chain": "Hoodi",
            "keystore": MAINNET_KEYSTORE,
            "keystorePassword": "not the password",
            "validatorIndex": 7,
        }}),
    )
    .unwrap_err();
    assert_eq!(wrong_password, json!("The keystore password is incorrect."));
}

#[test]
fn partial_deposit_and_keystore_commands() {
    let (_app, webview) = app();
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(
        invoke(
            &webview,
            "keystore_pubkey",
            json!({"path": MAINNET_KEYSTORE})
        ),
        Ok(json!(MAINNET_PUBKEY))
    );
    let not_a_keystore = dir.path().join("notes.json");
    std::fs::write(&not_a_keystore, "{}").unwrap();
    assert!(invoke(
        &webview,
        "keystore_pubkey",
        json!({"path": not_a_keystore.to_str().unwrap()})
    )
    .unwrap_err()
    .as_str()
    .unwrap()
    .ends_with("is not a valid keystore file."));

    let file = invoke(
        &webview,
        "generate_partial_deposit",
        json!({"request": {
            "folder": dir.path().to_str().unwrap(),
            "chain": "Gnosis",
            "keystore": MAINNET_KEYSTORE,
            "keystorePassword": "MyPasswordIs",
            "amount": "1.5",
            "withdrawalAddress": "0x00000000219ab540356cBB839Cbe05303d7705Fa",
            "compounding": true,
        }}),
    )
    .unwrap();
    let deposits: Value =
        serde_json::from_str(&std::fs::read_to_string(file.as_str().unwrap()).unwrap()).unwrap();
    // 1.5 GNO on Gnosis is a 48e9 gwei deposit message with compounding (0x02) credentials.
    assert_eq!(deposits[0]["amount"], json!(48_000_000_000u64));
    assert_eq!(deposits[0]["pubkey"], json!(MAINNET_PUBKEY));
    assert!(deposits[0]["withdrawal_credentials"]
        .as_str()
        .unwrap()
        .starts_with("02"));

    let file = invoke(
        &webview,
        "generate_bls_change_keystore",
        json!({"request": {
            "folder": dir.path().to_str().unwrap(),
            "chain": "Mainnet",
            "keystore": MAINNET_KEYSTORE,
            "keystorePassword": "MyPasswordIs",
            "validatorIndex": 1,
            "withdrawalAddress": "0x3434343434343434343434343434343434343434",
        }}),
    )
    .unwrap();
    let entry: Value =
        serde_json::from_str(&std::fs::read_to_string(file.as_str().unwrap()).unwrap()).unwrap();
    assert_eq!(entry["message"]["validator_index"], json!(1));
    assert_eq!(
        entry["message"]["to_execution_address"],
        json!("0x3434343434343434343434343434343434343434")
    );
}
