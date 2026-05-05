// TurboSpin (Spinoza) backend.
//
// Spawns the Spinoza CLI from the bundled `TurboSpin/` cargo workspace:
//
//     cargo run --release --quiet --manifest-path <…>/TurboSpin/Cargo.toml \
//         -p spinoza -- --qasm <tmp.qasm> --no-compression
//     cargo run --release --quiet --manifest-path <…>/TurboSpin/Cargo.toml \
//         -p spinoza -- --qasm <tmp.qasm> --compression-bits 4 --show-statevector
//
// The CLI prints a header (`source:`, `qubits:`, `dimension:`, `norm:`)
// followed by `statevector:` and a row per basis state, e.g.
//
//   0 | 00 | re=0.500000000000 | im=0.000000000000 | magnitude=… | probability=…
//
// Either part can be overridden:
//   * `$TURBOSPIN_MANIFEST`  full path to TurboSpin's `Cargo.toml`
//   * `$TURBOSPIN_CARGO`     name / path of the cargo binary (default `cargo`)
//
// All output is parsed in pure Rust into a `SimulationState`.

use std::path::PathBuf;
use std::process::Command;

use crate::state::TurboSpinCompression;
use crate::state::simulation::{Complex, SimulationState};

/// Default manifest sits next to the `qsim_ui` source tree:
///   `<this repo>/TurboSpin/Cargo.toml`
fn default_manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("TurboSpin/Cargo.toml")
}

fn manifest_path() -> PathBuf {
    std::env::var_os("TURBOSPIN_MANIFEST")
        .map(PathBuf::from)
        .unwrap_or_else(default_manifest)
}

fn cargo_bin() -> String {
    std::env::var("TURBOSPIN_CARGO").unwrap_or_else(|_| "cargo".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_qasm_source(
    source: &str,
    compression: TurboSpinCompression,
) -> Result<SimulationState, String> {
    let manifest = manifest_path();
    if !manifest.is_file() {
        return Err(format!(
            "TurboSpin manifest not found: {} (set $TURBOSPIN_MANIFEST)",
            manifest.display()
        ));
    }
    let workspace_dir = manifest
        .parent()
        .ok_or_else(|| "TurboSpin manifest has no parent dir".to_string())?
        .to_path_buf();

    let mut tmp_dir = std::env::temp_dir();
    tmp_dir.push("qsim_ui_turbospin");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("temp dir: {e}"))?;

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros())
        .unwrap_or(0);
    let qasm_path = tmp_dir.join(format!("circuit_{stamp}.qasm"));
    let sanitized = sanitize_for_spinoza(source);
    std::fs::write(&qasm_path, sanitized.as_bytes())
        .map_err(|e| format!("write qasm: {e}"))?;

    // Run *inside* the TurboSpin workspace so its `rust-toolchain.toml` pin
    // (currently nightly) is honoured. The qsim_ui workspace itself is on
    // stable.
    let out = Command::new(cargo_bin())
        .current_dir(&workspace_dir)
        .args([
            "run",
            "--release",
            "--quiet",
            "-p",
            "spinoza",
            "--",
        ])
        .arg("--qasm")
        .arg(&qasm_path)
        .args(runtime_args_for(compression))
        .output()
        .map_err(|e| format!("cargo: {e}"))?;

    let _ = std::fs::remove_file(&qasm_path);

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(first_useful_line(err.trim()));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    parse_spinoza_output(&stdout)
}

/// Reduce the editor buffer to what we feed Spinoza on disk. Lines are dropped when
/// the first token is one of the heads below (so `include` / `creg` / `measure` /
/// `gate` defs from `qelib1.inc` are not required after stripping). Lines starting
/// with `#` are dropped — strict OpenQASM 2 (and the `qasm` lexer) treat `#` as an
/// illegal character when comments use `#` instead of `//`.
fn sanitize_for_spinoza(src: &str) -> String {
    const DROP_HEADS: &[&str] = &[
        "include", "creg", "measure", "barrier", "reset", "if", "opaque", "gate",
    ];
    let mut out = String::with_capacity(src.len());
    for line in src.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            continue;
        }
        let head = trimmed
            .split(|c: char| c.is_ascii_whitespace() || c == '(')
            .next()
            .unwrap_or("");
        if DROP_HEADS.contains(&head) {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn runtime_args_for(compression: TurboSpinCompression) -> Vec<String> {
    match compression.bits() {
        None => vec!["--no-compression".to_string()],
        Some(bits) => vec![
            "--compression-bits".to_string(),
            bits.to_string(),
            "--show-statevector".to_string(),
        ],
    }
}

#[cfg(target_arch = "wasm32")]
pub fn run_qasm_source(
    _source: &str,
    _compression: TurboSpinCompression,
) -> Result<SimulationState, String> {
    Err("TurboSpin runs on desktop builds only".into())
}

fn first_useful_line(s: &str) -> String {
    // Cargo and panic messages bury the actual cause behind several lines of
    // boilerplate. Walk the stderr lines, drop common noise, and prefer the
    // last informative one.
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

/// Parse Spinoza's CLI report into a `SimulationState`. Tolerant of
/// surrounding cargo / progress noise.
fn parse_spinoza_output(stdout: &str) -> Result<SimulationState, String> {
    let mut num_qubits: Option<usize> = None;
    let mut amps: Vec<Complex> = Vec::new();
    let mut in_sv = false;

    for raw in stdout.lines() {
        let line = raw.trim();

        if let Some(rest) = line.strip_prefix("qubits:") {
            num_qubits = rest.trim().parse::<usize>().ok();
            continue;
        }
        if line.starts_with("statevector:") {
            in_sv = true;
            continue;
        }
        // Compression-report output begins with `bits | …`; a UI run may ask
        // either for exact `--no-compression` output or a compressed /
        // decompressed statevector via `--compression-bits`.
        if line.starts_with("bits |") || line.starts_with("compression") {
            in_sv = false;
            continue;
        }

        if in_sv {
            if let Some((re, im)) = parse_amplitude_line(line) {
                amps.push(Complex {
                    re: re as f32,
                    im: im as f32,
                });
            } else if line.is_empty() {
                // tolerate blank lines mid-block
                continue;
            } else {
                in_sv = false;
            }
        }
    }

    let n = num_qubits
        .ok_or_else(|| "spinoza: no `qubits:` line in output".to_string())?;
    let expected = 1usize << n;
    if amps.len() != expected {
        return Err(format!(
            "spinoza: shape mismatch — {} amplitudes for {} qubits (need {})",
            amps.len(),
            n,
            expected,
        ));
    }
    if n > 10 {
        return Err(format!("spinoza: {n} qubits exceeds UI limit of 10"));
    }

    Ok(SimulationState::from_statevector(n, amps))
}

/// Spinoza row format:
///   `<index> | <binary> | re=<f> | im=<f> | magnitude=<f> | probability=<f>`
fn parse_amplitude_line(line: &str) -> Option<(f64, f64)> {
    let mut re = None;
    let mut im = None;
    for tok in line.split('|') {
        let tok = tok.trim();
        if let Some(rest) = tok.strip_prefix("re=") {
            re = rest.trim().parse::<f64>().ok();
        } else if let Some(rest) = tok.strip_prefix("im=") {
            im = rest.trim().parse::<f64>().ok();
        }
    }
    Some((re?, im?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::TurboSpinCompression;

    const BELL_OUTPUT: &str = r#"source: QASM file /tmp/qsim_test.qasm
qubits: 2
dimension: 4
norm: 1.000000000000

statevector:
0 | 00 | re=0.707106781187 | im=0.000000000000 | magnitude=0.707106781187 | probability=0.500000000000
1 | 01 | re=0.000000000000 | im=0.000000000000 | magnitude=0.000000000000 | probability=0.000000000000
2 | 10 | re=0.000000000000 | im=0.000000000000 | magnitude=0.000000000000 | probability=0.000000000000
3 | 11 | re=0.707106781187 | im=0.000000000000 | magnitude=0.707106781187 | probability=0.500000000000
"#;

    #[test]
    fn parses_bell_pair_output() {
        let sim = parse_spinoza_output(BELL_OUTPUT).unwrap();
        assert_eq!(sim.num_qubits, 2);
        assert_eq!(sim.statevector.len(), 4);

        // |00⟩ and |11⟩ amplitudes ≈ 1/√2.
        let inv_sqrt2 = 0.5_f32.sqrt();
        assert!((sim.statevector[0].re - inv_sqrt2).abs() < 1e-4);
        assert!((sim.statevector[3].re - inv_sqrt2).abs() < 1e-4);
        assert!(sim.statevector[1].re.abs() < 1e-6);
        assert!(sim.statevector[2].re.abs() < 1e-6);

        // Probabilities derived in Rust.
        assert!((sim.probabilities[0] - 0.5).abs() < 1e-4);
        assert!((sim.probabilities[3] - 0.5).abs() < 1e-4);

        // Bell-pair single-qubit reduced state is maximally mixed.
        assert!(sim.bloch.len() == 2);
        for b in &sim.bloch {
            assert!(b.x.abs() < 1e-4);
            assert!(b.y.abs() < 1e-4);
            assert!(b.z.abs() < 1e-4);
        }
    }

    #[test]
    fn sanitizer_drops_unsupported_directives() {
        let src = "OPENQASM 2.0;\ninclude \"qelib1.inc\";\nqreg q[2];\ncreg c[2];\nh q[0];\ncx q[0], q[1];\nmeasure q[0] -> c[0];\n";
        let cleaned = sanitize_for_spinoza(src);
        assert!(!cleaned.contains("include"));
        assert!(!cleaned.contains("creg"));
        assert!(!cleaned.contains("measure"));
        assert!(cleaned.contains("qreg q[2];"));
        assert!(cleaned.contains("h q[0];"));
        assert!(cleaned.contains("cx q[0], q[1];"));
    }

    #[test]
    fn selects_expected_cli_args_for_compression_mode() {
        assert_eq!(
            runtime_args_for(TurboSpinCompression::Lossless),
            vec!["--no-compression"]
        );
        assert_eq!(
            runtime_args_for(TurboSpinCompression::Bits(4)),
            vec!["--compression-bits", "4", "--show-statevector"]
        );
    }
}
