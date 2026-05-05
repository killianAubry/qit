// Noise model configuration and application.
//
// Stores parameters for quantum noise simulation and applies them to
// statevectors via quantum-trajectory (Monte Carlo wavefunction) methods.
//
// Three noise channels are implemented:
//   * Depolarising  — probabilistic Pauli X/Y/Z
//   * Amplitude damping (T1) — energy relaxation |1⟩ → |0⟩
//   * Phase damping (T2) — pure dephasing (probabilistic Z)
//
// Per-gate custom noise is layered on top: for each gate in the circuit
// matching a key in `noise_per_gate`, additional depolarising + amplitude
// damping is applied with the gate-specific parameters.

use std::collections::HashMap;

use rand::Rng;

use crate::state::circuit::Circuit;
use crate::state::simulation::Complex;

#[derive(Clone, Debug)]
pub struct GateNoiseParams {
    /// Per-gate depolarising probability (0.0–1.0).
    pub depolarizing_prob: f32,
    /// Per-gate amplitude-damping gamma (0.0–1.0).
    pub damping_gamma: f32,
}

impl Default for GateNoiseParams {
    fn default() -> Self {
        Self {
            depolarizing_prob: 0.001,
            damping_gamma: 0.001,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationSource {
    None,
    Qiskit,
    Cirq,
    Custom,
}

impl CalibrationSource {
    pub fn label(&self) -> &'static str {
        match self {
            CalibrationSource::None => "none",
            CalibrationSource::Qiskit => "qiskit",
            CalibrationSource::Cirq => "cirq",
            CalibrationSource::Custom => "custom",
        }
    }
}

#[derive(Clone, Debug)]
pub struct NoiseConfig {
    // ── Noise-model toggles & parameters ──

    /// Global toggle: apply noise models during simulation.
    pub noise_enabled: bool,

    /// Depolarising channel: replaces the state with a maximally-mixed
    /// state with probability `depolarizing_probability / 4` per qubit.
    pub depolarizing_enabled: bool,
    pub depolarizing_probability: f32,

    /// Amplitude damping (energy relaxation, T1): models spontaneous
    /// emission from |1⟩ → |0⟩. Gamma = 1 - exp(-Δt/T1).
    pub amplitude_damping_enabled: bool,
    pub amplitude_damping_gamma: f32,

    /// Phase damping (pure dephasing, T2): models loss of phase coherence
    /// without energy loss. Gamma = 1 - exp(-Δt/T2).
    pub phase_damping_enabled: bool,
    pub phase_damping_gamma: f32,

    // ── Per-gate custom noise ──
    pub noise_per_gate: HashMap<String, GateNoiseParams>,

    // ── Device calibration ──
    pub device_t1_us: f32,
    pub device_t2_us: f32,
    pub device_gate_error_rate: f32,
    pub device_readout_error_rate: f32,
    pub device_qubit_frequency_ghz: f32,
    pub calibration_source: CalibrationSource,

    // ── Import status ──
    pub calibration_imported: bool,
    pub calibration_device_name: String,
}

impl Default for NoiseConfig {
    fn default() -> Self {
        Self {
            noise_enabled: false,
            depolarizing_enabled: true,
            depolarizing_probability: 0.01,
            amplitude_damping_enabled: true,
            amplitude_damping_gamma: 0.005,
            phase_damping_enabled: true,
            phase_damping_gamma: 0.003,
            noise_per_gate: HashMap::new(),
            device_t1_us: 50.0,
            device_t2_us: 70.0,
            device_gate_error_rate: 0.001,
            device_readout_error_rate: 0.01,
            device_qubit_frequency_ghz: 5.0,
            calibration_source: CalibrationSource::None,
            calibration_imported: false,
            calibration_device_name: String::new(),
        }
    }
}

impl NoiseConfig {
    /// Seed known gate names into the per-gate config map so the UI always
    /// has entries to display.
    pub fn ensure_default_gates(&mut self) {
        let defaults: &[&str] = &["H", "X", "Y", "Z", "S", "T", "RX", "RY", "RZ", "CX", "CZ", "SWAP"];
        for &name in defaults {
            self.noise_per_gate
                .entry(name.to_string())
                .or_insert_with(GateNoiseParams::default);
        }
    }
}

// ── Noise application ──────────────────────────────────────────────────

/// Apply noise channels to a statevector in-place.
///
/// Called after the backend returns a clean statevector.  Mutates the
/// amplitudes so every downstream panel (probability, Bloch, state vector)
/// sees the noisy result automatically.
pub fn apply_noise(
    statevector: &mut [Complex],
    num_qubits: usize,
    config: &NoiseConfig,
    circuit: &Circuit,
) {
    if !config.noise_enabled {
        return;
    }
    let dim = statevector.len();
    if num_qubits == 0 || dim == 0 || dim != 1 << num_qubits {
        return;
    }

    let mut rng = rand::thread_rng();

    // ── Per-qubit global noise channels ──
    for qubit in 0..num_qubits {
        if config.depolarizing_enabled {
            apply_depolarizing(statevector, num_qubits, qubit, config.depolarizing_probability, &mut rng);
        }
        if config.phase_damping_enabled {
            apply_phase_damping(statevector, num_qubits, qubit, config.phase_damping_gamma, &mut rng);
        }
        if config.amplitude_damping_enabled {
            apply_amplitude_damping(statevector, num_qubits, qubit, config.amplitude_damping_gamma, &mut rng);
        }
    }

    // ── Per-gate noise ──
    // Each gate in the circuit may carry extra depolarising + amplitude damping.
    if !config.noise_per_gate.is_empty() {
        for gate in &circuit.gates {
            let label = &gate.label;
            if let Some(params) = config.noise_per_gate.get(label) {
                let q = gate.qubit;
                if q < num_qubits {
                    if params.depolarizing_prob > 0.0 {
                        apply_depolarizing(statevector, num_qubits, q, params.depolarizing_prob, &mut rng);
                    }
                    if params.damping_gamma > 0.0 {
                        apply_amplitude_damping(statevector, num_qubits, q, params.damping_gamma, &mut rng);
                    }
                }
            }
        }
    }

    // ── Readout error ──
    // Flips measurement outcome with probability `device_readout_error_rate`.
    // We model this as a bit-flip (X) on each qubit.
    for qubit in 0..num_qubits {
        if config.device_readout_error_rate > 0.0 && rng.gen::<f32>() < config.device_readout_error_rate {
            apply_pauli_x(statevector, num_qubits, qubit);
        }
    }
}

// ── Channel implementations ────────────────────────────────────────────

/// Depolarising channel: with probability `p`, apply X, Y, or Z uniformly.
fn apply_depolarizing(
    sv: &mut [Complex],
    num_qubits: usize,
    qubit: usize,
    p: f32,
    rng: &mut impl Rng,
) {
    if p <= 0.0 {
        return;
    }
    if rng.gen::<f32>() >= p {
        return;
    }
    // Choose Pauli uniformly.
    match rng.gen_range(0u8..3u8) {
        0 => apply_pauli_x(sv, num_qubits, qubit),
        1 => apply_pauli_y(sv, num_qubits, qubit),
        _ => apply_pauli_z(sv, num_qubits, qubit),
    }
}

/// Phase damping: with probability `gamma`, apply Z (phase flip on |1⟩).
fn apply_phase_damping(
    sv: &mut [Complex],
    num_qubits: usize,
    qubit: usize,
    gamma: f32,
    rng: &mut impl Rng,
) {
    if gamma <= 0.0 {
        return;
    }
    if rng.gen::<f32>() < gamma {
        apply_pauli_z(sv, num_qubits, qubit);
    }
}

/// Amplitude damping via quantum trajectory.
///
/// Two branches:
///   No-jump (E0): scale |1⟩ amplitudes by √(1-γ), then renormalise.
///   Jump    (E1): project |1⟩ → |0⟩, then renormalise.
///
/// The branch is chosen randomly with p_jump = γ · ⟨ψ|1⟩⟨1|ψ⟩.
fn apply_amplitude_damping(
    sv: &mut [Complex],
    _num_qubits: usize,
    qubit: usize,
    gamma: f32,
    rng: &mut impl Rng,
) {
    if gamma <= 0.0 {
        return;
    }
    let dim = sv.len();
    let bit = 1usize << qubit;

    // Probability that qubit is in |1⟩.
    let mut p_one = 0.0_f32;
    for idx in 0..dim {
        if (idx & bit) != 0 {
            let c = sv[idx];
            p_one += c.re * c.re + c.im * c.im;
        }
    }

    let p_jump = gamma * p_one;
    if p_jump <= 0.0 {
        return;
    }

    if rng.gen::<f32>() < p_jump {
        // Jump: |1⟩ → |0⟩
        let mut buf = vec![Complex::default(); dim];
        for idx in 0..dim {
            if (idx & bit) != 0 {
                let target = idx ^ bit; // same index with qubit flipped to 0
                buf[target] = sv[idx];
            }
        }
        sv.copy_from_slice(&buf);
    } else {
        // No-jump: damp |1⟩ amplitudes.
        let scale = (1.0 - gamma).sqrt();
        for idx in 0..dim {
            if (idx & bit) != 0 {
                sv[idx].re *= scale;
                sv[idx].im *= scale;
            }
        }
    }

    renormalise(sv);
}

// ── Pauli gates on statevector ─────────────────────────────────────────

fn apply_pauli_x(sv: &mut [Complex], num_qubits: usize, qubit: usize) {
    let bit = 1usize << qubit;
    let dim = 1usize << num_qubits;
    for idx in 0..dim {
        if (idx & bit) == 0 {
            sv.swap(idx, idx | bit);
        }
    }
}

fn apply_pauli_y(sv: &mut [Complex], num_qubits: usize, qubit: usize) {
    let bit = 1usize << qubit;
    let dim = 1usize << num_qubits;
    // Y = -iXZ: flip bit AND conditional phase, then multiply |0→1⟩ by i,
    // |1→0⟩ by -i.
    for idx in 0..dim {
        if (idx & bit) == 0 {
            let j = idx | bit;
            // a|0⟩ + b|1⟩  →  -b*|0⟩ + a*|1⟩   … no.
            // Y|0⟩ = i|1⟩,  Y|1⟩ = -i|0⟩
            // So: amplitude from |0⟩ moves to |1⟩ multiplied by i.
            //     amplitude from |1⟩ moves to |0⟩ multiplied by -i.
            let amp_0 = sv[idx];
            let amp_1 = sv[j];
            // new |0⟩ = -i * amp_1
            sv[idx] = Complex { re: amp_1.im, im: -amp_1.re };
            // new |1⟩ = i * amp_0
            sv[j] = Complex { re: -amp_0.im, im: amp_0.re };
        }
    }
}

fn apply_pauli_z(sv: &mut [Complex], num_qubits: usize, qubit: usize) {
    let bit = 1usize << qubit;
    let dim = 1usize << num_qubits;
    for idx in 0..dim {
        if (idx & bit) != 0 {
            sv[idx].re = -sv[idx].re;
            sv[idx].im = -sv[idx].im;
        }
    }
}

// ── Utility ────────────────────────────────────────────────────────────

/// Renormalise a statevector in-place so that Σ|c|² = 1.
fn renormalise(sv: &mut [Complex]) {
    let norm_sq: f32 = sv.iter().map(|c| c.re * c.re + c.im * c.im).sum();
    if norm_sq > 1e-30 {
        let inv = 1.0 / norm_sq.sqrt();
        for c in sv.iter_mut() {
            c.re *= inv;
            c.im *= inv;
        }
    }
}
