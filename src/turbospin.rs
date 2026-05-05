// TurboSpin (Spinoza + optional BACQS) backend.
//
// Spawns the Spinoza CLI from the bundled `TurboSpin/` cargo workspace:
//
//     cargo run --release --quiet -p spinoza --bin spinoza -- \
//         --qasm <tmp.qasm> --comp-bit 0
//     cargo run --release --quiet -p spinoza --bin spinoza -- \
//         --qasm <tmp.qasm> --comp-bit 4
//
// `--comp-bit 0` runs raw Spinoza; `1..=8` runs the hybrid Clifford-tableau
// compressor path. The CLI prints one row per basis state (no header), e.g.
//
//   0 | 00 | re=+0.500000000000 | im=+0.000000000000 | magnitude=… | probability=…
//
// Legacy Spinoza runs that still emit `qubits:` / `statevector:` blocks are also
// accepted when parsing stdout.
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
use crate::state::metrics::CompressionInfo;

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

/// Result of a TurboSpin run: the simulation state plus optional compression metadata
/// when the CLI prints compression report lines (legacy); the current bundled binary
/// does not, so this is usually `None` for compressed runs.
pub struct TurboSpinResult {
    pub simulation: SimulationState,
    pub compression: Option<CompressionInfo>,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_qasm_source(
    source: &str,
    compression: TurboSpinCompression,
) -> Result<TurboSpinResult, String> {
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

fn comp_bit_args(compression: TurboSpinCompression) -> Vec<String> {
    let bits = compression.bits().unwrap_or(0);
    vec![
        "--comp-bit".to_string(),
        bits.to_string(),
    ]
}

#[cfg(target_arch = "wasm32")]
pub fn run_qasm_source(
    _source: &str,
    _compression: TurboSpinCompression,
) -> Result<TurboSpinResult, String> {
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

/// Parse Spinoza CLI stdout into a `TurboSpinResult`.
///
/// Supports the current binary (plain amplitude rows only) and legacy layouts that
/// include `qubits:` / `statevector:` headers and optional compression metadata.
fn parse_spinoza_output(stdout: &str, compression: TurboSpinCompression) -> Result<TurboSpinResult, String> {
    let mut declared_qubits: Option<usize> = None;
    let mut amps: Vec<Complex> = Vec::new();

    let mut compression_ratio: Option<f64> = None;
    let mut compression_fidelity: Option<f64> = None;
    let mut compression_norm_error: Option<f64> = None;
    let mut compressed_payload_bytes: Option<usize> = None;
    let mut compressed_metadata_bytes: Option<usize> = None;
    let mut compression_bits: Option<u8> = None;

    for raw in stdout.lines() {
        let line = raw.trim();

        if let Some(rest) = line.strip_prefix("qubits:") {
            declared_qubits = rest.trim().parse::<usize>().ok();
            continue;
        }

        if line.starts_with("statevector:")
            || line.starts_with("source:")
            || line.starts_with("dimension:")
            || line.starts_with("norm:")
            || line.starts_with("bits |")
            || line.starts_with("compression_report")
        {
            continue;
        }

        if let Some(rest) = line.strip_prefix("compression_ratio:") {
            compression_ratio = rest.trim().parse::<f64>().ok();
            continue;
        }
        if let Some(rest) = line.strip_prefix("compression_fidelity:") {
            compression_fidelity = rest.trim().parse::<f64>().ok();
            continue;
        }
        if let Some(rest) = line.strip_prefix("compression_norm_error:") {
            compression_norm_error = rest.trim().parse::<f64>().ok();
            continue;
        }
        if let Some(rest) = line.strip_prefix("compressed_payload_bytes:") {
            compressed_payload_bytes = rest.trim().parse::<usize>().ok();
            continue;
        }
        if let Some(rest) = line.strip_prefix("compressed_metadata_bytes:") {
            compressed_metadata_bytes = rest.trim().parse::<usize>().ok();
            continue;
        }
        if let Some(rest) = line.strip_prefix("compression_bits:") {
            compression_bits = rest.trim().parse::<u8>().ok();
            continue;
        }

        if let Some((re, im)) = parse_amplitude_line(line) {
            amps.push(Complex {
                re: re as f32,
                im: im as f32,
            });
        }
    }

    if amps.is_empty() {
        return Err("spinoza: no statevector rows in output".to_string());
    }
    if !amps.len().is_power_of_two() {
        return Err(format!(
            "spinoza: expected a power-of-two number of amplitudes (got {})",
            amps.len()
        ));
    }
    let n = amps.len().ilog2() as usize;

    if let Some(q) = declared_qubits {
        if q != n {
            return Err(format!(
                "spinoza: `qubits:` {q} disagrees with {} amplitudes ({n} qubits)",
                amps.len()
            ));
        }
    }
    if n > 20 {
        return Err(format!("spinoza: {n} qubits exceeds UI limit of 20"));
    }

    let simulation = SimulationState::from_statevector(n, amps);

    let comp_info = match compression.bits() {
        Some(_bits) => {
            match (compression_ratio, compression_fidelity, compressed_payload_bytes, compression_bits) {
                (Some(ratio), Some(fid), Some(payload), Some(bits)) => Some(CompressionInfo {
                    ratio,
                    fidelity: fid,
                    norm_error: compression_norm_error.unwrap_or(0.0),
                    payload_bytes: payload,
                    metadata_bytes: compressed_metadata_bytes.unwrap_or(0),
                    bits,
                }),
                _ => None,
            }
        }
        None => None,
    };

    Ok(TurboSpinResult { simulation, compression: comp_info })
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
        let result = parse_spinoza_output(BELL_OUTPUT, TurboSpinCompression::Lossless).unwrap();
        let sim = &result.simulation;
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

        // Lossless mode should not report compression metadata.
        assert!(result.compression.is_none());
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
    fn selects_expected_comp_bit_args() {
        assert_eq!(
            comp_bit_args(TurboSpinCompression::Lossless),
            vec!["--comp-bit".to_string(), "0".to_string()]
        );
        assert_eq!(
            comp_bit_args(TurboSpinCompression::Bits(4)),
            vec!["--comp-bit".to_string(), "4".to_string()]
        );
    }

    #[test]
    fn parses_legacy_compression_metadata_when_present() {
        let compressed_output = r#"source: QASM file /tmp/test.qasm
qubits: 2
dimension: 4
norm: 0.999987234567

compression_ratio: 3.450000
compression_fidelity: 0.998543
compression_norm_error: 1.277e-05
compressed_payload_bytes: 8
compressed_metadata_bytes: 29
compression_bits: 4

statevector:
0 | 00 | re=0.706890123456 | im=0.000000000000 | magnitude=0.706890123456 | probability=0.499695000000
1 | 01 | re=0.000000000000 | im=0.000000000000 | magnitude=0.000000000000 | probability=0.000000000000
2 | 10 | re=0.000000000000 | im=0.000000000000 | magnitude=0.000000000000 | probability=0.000000000000
3 | 11 | re=0.706890123456 | im=0.000000000000 | magnitude=0.706890123456 | probability=0.499695000000
"#;
        let result = parse_spinoza_output(compressed_output, TurboSpinCompression::Bits(4)).unwrap();
        assert_eq!(result.simulation.num_qubits, 2);

        let comp = result.compression.expect("should have compression metadata");
        assert!((comp.ratio - 3.45).abs() < 0.01);
        assert!((comp.fidelity - 0.998543).abs() < 1e-5);
        assert_eq!(comp.payload_bytes, 8);
        assert_eq!(comp.metadata_bytes, 29);
        assert_eq!(comp.bits, 4);
    }

    #[test]
    fn plain_cli_rows_have_no_compression_metadata() {
        let out = r#"0 | 00 | re=+0.707106781187 | im=+0.000000000000 | magnitude=0.707106781187 | probability=0.500000000000
1 | 01 | re=+0.000000000000 | im=+0.000000000000 | magnitude=0.000000000000 | probability=0.000000000000
2 | 10 | re=+0.000000000000 | im=+0.000000000000 | magnitude=0.000000000000 | probability=0.000000000000
3 | 11 | re=+0.707106781187 | im=+0.000000000000 | magnitude=0.707106781187 | probability=0.500000000000
"#;
        let result = parse_spinoza_output(out, TurboSpinCompression::Bits(4)).unwrap();
        assert_eq!(result.simulation.num_qubits, 2);
        assert!(result.compression.is_none());
    }
}
