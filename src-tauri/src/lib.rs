//! Tauri shell of Wagyu Key Gen. All key handling lives in `wagyu-core`; this crate only wires
//! commands, plugins and window behaviour (ported from `src/electron/index.ts`).

mod commands;

use tauri::{Manager, RunEvent};
use tauri_plugin_clipboard_manager::ClipboardExt;

/// Short git commit hash baked in by `build.rs`.
pub const COMMIT_HASH: &str = env!("WAGYU_COMMIT");

/// Registers every IPC command on `builder`. Generic over the runtime so the same wiring is used
/// by the real application and by the mock-runtime tests in `tests/ipc.rs`.
pub fn register_commands<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        commands::create_mnemonic,
        commands::validate_mnemonic,
        commands::generate_keys,
        commands::validate_bls_credentials,
        commands::generate_bls_change,
        commands::is_address,
        commands::does_directory_exist,
        commands::is_directory_writable,
        commands::find_first_file,
        commands::version_info,
        commands::quit,
    ])
}

pub fn run() {
    register_commands(tauri::Builder::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let info = app.package_info();
            let title = format!("{} {}-{}", info.name, info.version, COMMIT_HASH);
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title(&title);
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building the Wagyu Key Gen application")
        .run(|app, event| {
            // Clear the clipboard on quit so a copied mnemonic or password does not outlive the
            // app. This must happen on `ExitRequested`, not `Exit`: the clipboard-manager
            // plugin's own internal `RunEvent::Exit` handler takes and drops its
            // `arboard::Clipboard` right before ours would run (arboard requires that drop to
            // flush a write to the OS clipboard, and Tauri runs a plugin's `on_event` before the
            // app's own `.run` callback for the same event). Clearing here at `Exit` finds the
            // clipboard already taken and panics silently, so nothing gets cleared.
            if let RunEvent::ExitRequested { .. } = event {
                if let Err(err) = app.clipboard().clear() {
                    eprintln!("failed to clear the clipboard on exit: {err}");
                }
            }
        });
}
