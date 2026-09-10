#![allow(dead_code)]

use std::path::{Path, PathBuf};

use alloy_primitives::U256;
use serde_json::Value;

pub const TEST_VECTOR_PASSWORD: &str = "𝔱𝔢𝔰𝔱𝔭𝔞𝔰𝔰𝔴𝔬𝔯𝔡🔑";
pub const TEST_VECTOR_SECRET: &str =
    "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f";
pub const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
pub const SISTER: &str = "sister protect peanut hill ready work profit fit wish want small inflict flip member tail between sick setup bright duck morning sell paper worry";

pub fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("vectors")
}

pub fn golden_dir(case: &str) -> PathBuf {
    vectors_dir().join("golden").join(case)
}

pub fn read_json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("cannot parse {}: {e}", path.display()))
}

pub fn read_vector(name: &str) -> Value {
    read_json(&vectors_dir().join(name))
}

/// Python integers in the vectors exceed u64; serde_json keeps their digits (arbitrary_precision).
pub fn big_int_to_32_bytes(value: &Value) -> [u8; 32] {
    let digits = match value {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        other => panic!("not an integer: {other}"),
    };
    U256::from_str_radix(&digits, 10)
        .expect("decimal integer")
        .to_be_bytes::<32>()
}

pub fn hex_to_bytes(value: &Value) -> Vec<u8> {
    let s = value.as_str().expect("hex string");
    hex::decode(s.strip_prefix("0x").unwrap_or(s)).expect("valid hex")
}

/// All golden case directories whose params.json has the given `command`.
pub fn golden_cases(command: &str) -> Vec<PathBuf> {
    let mut cases: Vec<PathBuf> = std::fs::read_dir(vectors_dir().join("golden"))
        .expect("golden dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .filter(|p| read_json(&p.join("params.json"))["command"] == command)
        .collect();
    cases.sort();
    assert!(!cases.is_empty(), "no golden cases for {command}");
    cases
}

#[cfg(unix)]
pub fn assert_mode_0400(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o400, "{} has mode {mode:o}", path.display());
}

#[cfg(not(unix))]
pub fn assert_mode_0400(_path: &Path) {}
