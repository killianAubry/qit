// OldTurboSpin (legacy Spinoza snapshot) backend.
//
// Identical to the TurboSpin backend but targets the `OldTurboSpin/` cargo
// workspace instead. The output format is the same; parsing is shared.
//
// Either part can be overridden:
//   * `$OLDTURBOSPIN_MANIFEST`  full path to OldTurboSpin's `Cargo.toml`
//   * `$OLDTURBOSPIN_CARGO`     name / path of the cargo binary (default `cargo`)

use std::path::PathBuf;
use std::process::Command;

use crate::state::TurboSpinCompression;

/// Shared parse logic lives in `turbospin` — both backends consume the same
/// Spinoza stdout format.
use crate::turbospin::{TurboSpinResult, parse_spinoza_output};

/// Default manifest sits next to the `qsim_ui` source tree:
///   `<this repo>/OldTurboSpin/Cargo.toml`
fn default_manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("OldTurboSpin/Cargo.toml")
}

fn manifest_path() -> PathBuf {
    std::env::var_os("OLDTURBOSPIN_MANIFEST")
        .map(PathBuf::from)
        .unwrap_or_else(default_manifest)
}

fn cargo_bin() -> String {
    std::env::var("OLDTURBOSPIN_CARGO").unwrap_or_else(|_| "cargo".to_string())
}

/// Sanitize OpenQASM source for Spinoza — shared with the main turbospin module.
use crate::turbospin::sanitize_for_spinoza;

#[cfg(not(target_arch = "wasm32"))]
pub fn run_qasm_source(
    source: &str,
    compression: TurboSpinCompression,
) -> Result<TurboSpinResult, String> {
    let manifest = manifest_path();
    if !manifest.is_file() {
        return Err(format!(
            "OldTurboSpin manifest not found: {} (set $OLDTURBOSPIN_MANIFEST)",
            manifest.display()
        ));
    }
    let workspace_dir = manifest
        .parent()
        .ok_or_else(|| "OldTurboSpin manifest has no parent dir".to_string())?
        .to_path_buf();

    let mut tmp_dir = std::env::temp_dir();
    tmp_dir.push("qsim_ui_oldturbospin");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("temp dir: {e}"))?;

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros())
        .unwrap_or(0);
    let qasm_path = tmp_dir.join(format!("circuit_{stamp}.qasm"));
    let sanitized = sanitize_for_spinoza(source);
    std::fs::write(&qasm_path, sanitized.as_bytes())
        .map_err(|e| format!("write qasm: {e}"))?;

    let out = Command::new(cargo_bin())
        .current_dir(&workspace_dir)
        .args([
            "run",
            "--release",
            "--quiet",
            "-p",
            "spinoza",
            "--bin",
            "spinoza",
            "--",
        ])
        .arg("--qasm")
        .arg(&qasm_path)
        .args(comp_bit_args(compression))
        .output()
        .map_err(|e| format!("cargo: {e}"))?;

    let _ = std::fs::remove_file(&qasm_path);

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(first_useful_line(err.trim()));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    parse_spinoza_output(&stdout, compression)
}

#[cfg(target_arch = "wasm32")]
pub fn run_qasm_source(
    _source: &str,
    _compression: TurboSpinCompression,
) -> Result<TurboSpinResult, String> {
    Err("OldTurboSpin runs on desktop builds only".into())
}

fn comp_bit_args(compression: TurboSpinCompression) -> Vec<String> {
    let bits = compression.bits().unwrap_or(0);
    vec![
        "--comp-bit".to_string(),
        bits.to_string(),
    ]
}

fn first_useful_line(s: &str) -> String {
    let candidates: Vec<&str> = s
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .filter(|l| {
            !(l.starts_with("note:")
                || l.starts_with("help:")
                || l.starts_with("warning:")
                || l.contains("backtrace")
                || l.contains("`#[warn"))
        })
        .collect();
    candidates
        .iter()
        .rev()
        .copied()
        .find(|l| l.contains(':'))
        .or_else(|| candidates.last().copied())
        .unwrap_or("")
        .to_string()
}
