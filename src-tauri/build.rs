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
    tauri_build::build()
}
