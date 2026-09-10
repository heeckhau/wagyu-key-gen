fn main() {
    // Short hash of the last non-merge commit, shown in the window title and the footer
    // (replaces git-revision-webpack-plugin).
    let commit = std::process::Command::new("git")
        .args([
            "rev-list",
            "--max-count=1",
            "--no-merges",
            "--abbrev-commit",
            "HEAD",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=WAGYU_COMMIT={commit}");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs");

    // tauri-build normally embeds the Windows app manifest itself, but it does so with a
    // `cargo:rustc-link-arg-bins` linker arg, which only applies to the crate's own [[bin]]
    // target and never to [[test]] targets. Without the manifest, `src-tauri/tests/ipc.rs`
    // (which uses tauri::test's mock runtime) fails to even start on Windows with
    // `STATUS_ENTRYPOINT_NOT_FOUND` -- see https://github.com/tauri-apps/tauri/issues/13419.
    // Work around it by disabling tauri-build's own (bins-only) manifest embedding and doing it
    // ourselves with a plain `cargo:rustc-link-arg`, which applies to every compiled artifact,
    // including tests.
    let attributes = tauri_build::Attributes::new();
    #[cfg(windows)]
    let attributes = {
        embed_windows_manifest();
        attributes.windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest())
    };
    tauri_build::try_build(attributes).expect("failed to run tauri-build");
}

/// Embeds `windows-app-manifest.xml` (declaring the Common Controls v6 dependency Tauri's
/// `common-controls-v6` feature needs) into every artifact this crate produces, unlike
/// tauri-build's own manifest embedding which only reaches the app binary.
#[cfg(windows)]
fn embed_windows_manifest() {
    let manifest = std::env::current_dir()
        .unwrap()
        .join("windows-app-manifest.xml");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg=/MANIFESTINPUT:{}",
        manifest.to_str().unwrap()
    );
    // Turn linker warnings (e.g. a malformed manifest) into build errors instead of a silently
    // broken binary.
    println!("cargo:rustc-link-arg=/WX");
}
