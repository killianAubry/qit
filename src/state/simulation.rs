// Result of a simulation run.
//
// The **state vector** is the source of truth: every other panel — probability
// distribution, Bloch spheres — is derived from it in `from_statevector`. This
// means each backend (Qiskit, TurboSpin/Spinoza, mock) only has to produce a
// `Vec<Complex>`; the visualizer code is identical regardless of source.
//
// Conventions
//   * `statevector.len()  == 2.pow(num_qubits)`
//   * `bloch.len()        == num_qubits`
//   * `probabilities[i]   == |statevector[i]|²`
//   * Basis ordering is little-endian: bit `i` of the basis index is qubit `i`
//     (matches Qiskit and Spinoza).

use crate::state::metrics::Metrics;

#[derive(Clone, Debug)]
pub struct SimulationState {
    pub num_qubits: usize,
    /// 2^num_qubits amplitudes. The single source of truth for downstream
    /// panels.
    pub statevector: Vec<Complex>,
    /// Cached `|amp|²` per basis state.
    pub probabilities: Vec<f32>,
    /// Cached per-qubit reduced-state Bloch vector.
    pub bloch: Vec<BlochVector>,
    pub metrics: Metrics,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Complex {
    pub re: f32,
    pub im: f32,
}

impl Complex {
    pub fn norm_sqr(self) -> f32 {
        self.re * self.re + self.im * self.im
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BlochVector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl SimulationState {
    /// Build a simulation snapshot from a freshly-produced statevector.
    /// `num_qubits` must satisfy `statevector.len() == 1 << num_qubits`.
    pub fn from_statevector(num_qubits: usize, statevector: Vec<Complex>) -> Self {
        let probabilities = statevector.iter().map(|c| c.norm_sqr()).collect();
        let bloch = compute_bloch(&statevector, num_qubits);
        Self {
            num_qubits,
            statevector,
            probabilities,
            bloch,
            metrics: Metrics::default(),
        }
    }

    /// |0…0⟩ — used at boot or after `:clear` so panels render zeros instead
    /// of the previous run's data.
    pub fn ground_state(num_qubits: usize) -> Self {
        let n = num_qubits.clamp(1, 20);
        let dim = 1usize << n;
        let mut sv = vec![Complex::default(); dim];
        sv[0] = Complex { re: 1.0, im: 0.0 };
        Self::from_statevector(n, sv)
    }

    /// Deterministic, visually-interesting fake statevector. Useful for UI
    /// development and as a placeholder before any real run.
    #[allow(dead_code)]
    pub fn mock(num_qubits: usize) -> Self {
        let n = num_qubits.clamp(1, 20);
        let dim = 1usize << n;

        let mut sv = vec![Complex::default(); dim];
        for i in 0..dim {
            let theta = (i as f32) * 0.713 + 0.41;
            let weight = 1.0 - 0.55 * (i as f32 / dim.max(1) as f32);
            sv[i] = Complex {
                re: theta.cos() * weight,
                im: theta.sin() * (0.4 + 0.6 * weight),
            };
        }
        let total: f32 = sv.iter().map(|c| c.norm_sqr()).sum();
        let inv_norm = 1.0 / total.sqrt().max(1e-9);
        for c in &mut sv {
            c.re *= inv_norm;
            c.im *= inv_norm;
        }

        Self::from_statevector(n, sv)
    }
}

/// Per-qubit Bloch vectors derived **only** from the statevector (partial
/// trace of the pure state). UI panels call this so spheres always match the
/// current SV, independent of any cached `simulation.bloch`.
pub fn bloch_from_statevector(sv: &[Complex], num_qubits: usize) -> Vec<BlochVector> {
    compute_bloch(sv, num_qubits)
}

/// Compute per-qubit reduced-state Bloch vectors directly from a pure-state
/// statevector, without ever materialising the full N-qubit density matrix.
///
/// For each qubit `i` we partial-trace every other qubit; the 2×2 block
/// `ρᵢ = Σ_rest ψ[rest, qᵢ=a] · ψ[rest, qᵢ=b]*`  decomposes into  
/// `r_x =  2·Re(ρᵢ[0,1])`, `r_y = -2·Im(ρᵢ[0,1])`, `r_z =  ρᵢ[0,0] − ρᵢ[1,1]`.
fn compute_bloch(sv: &[Complex], num_qubits: usize) -> Vec<BlochVector> {
    if num_qubits == 0 || sv.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(num_qubits);
    let dim = sv.len();
    for i in 0..num_qubits {
        let bit = 1usize << i;
        let mut r00 = 0.0_f32;
        let mut r11 = 0.0_f32;
        let mut r01_re = 0.0_f32;
        let mut r01_im = 0.0_f32;

        for x in 0..dim {
            let a = sv[x];
            let mag = a.re * a.re + a.im * a.im;
            if (x & bit) == 0 {
                r00 += mag;
                let b = sv[x | bit];
                // ψ[bit=0] · conj(ψ[bit=1])
                r01_re += a.re * b.re + a.im * b.im;
                r01_im += a.im * b.re - a.re * b.im;
            } else {
                r11 += mag;
            }
        }

        out.push(BlochVector {
            x: 2.0 * r01_re,
            y: -2.0 * r01_im,
            z: r00 - r11,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{BlochVector, Complex, SimulationState};

    fn approx_eq(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-4, "expected {a} ~= {b}");
    }

    fn assert_bloch(v: BlochVector, x: f32, y: f32, z: f32) {
        approx_eq(v.x, x);
        approx_eq(v.y, y);
        approx_eq(v.z, z);
    }

    #[test]
    fn single_qubit_basis_and_phase_states() {
        let zero = SimulationState::from_statevector(
            1,
            vec![Complex { re: 1.0, im: 0.0 }, Complex { re: 0.0, im: 0.0 }],
        );
        assert_bloch(zero.bloch[0], 0.0, 0.0, 1.0);

        let one = SimulationState::from_statevector(
            1,
            vec![Complex { re: 0.0, im: 0.0 }, Complex { re: 1.0, im: 0.0 }],
        );
        assert_bloch(one.bloch[0], 0.0, 0.0, -1.0);

        let inv_sqrt2 = 0.5_f32.sqrt();
        let plus = SimulationState::from_statevector(
            1,
            vec![
                Complex {
                    re: inv_sqrt2,
                    im: 0.0,
                },
                Complex {
                    re: inv_sqrt2,
                    im: 0.0,
                },
            ],
        );
        assert_bloch(plus.bloch[0], 1.0, 0.0, 0.0);

        let plus_i = SimulationState::from_statevector(
            1,
            vec![
                Complex {
                    re: inv_sqrt2,
                    im: 0.0,
                },
                Complex {
                    re: 0.0,
                    im: inv_sqrt2,
                },
            ],
        );
        assert_bloch(plus_i.bloch[0], 0.0, 1.0, 0.0);
    }

    #[test]
    fn two_qubit_product_state_respects_little_endian_qubit_order() {
        let inv_sqrt2 = 0.5_f32.sqrt();
        // |+0> in little-endian indexing:
        // basis order |q1 q0> => [|00>, |01>, |10>, |11>]
        let sim = SimulationState::from_statevector(
            2,
            vec![
                Complex {
                    re: inv_sqrt2,
                    im: 0.0,
                },
                Complex {
                    re: inv_sqrt2,
                    im: 0.0,
                },
                Complex { re: 0.0, im: 0.0 },
                Complex { re: 0.0, im: 0.0 },
            ],
        );
        assert_bloch(sim.bloch[0], 1.0, 0.0, 0.0);
        assert_bloch(sim.bloch[1], 0.0, 0.0, 1.0);
    }

    #[test]
    fn bell_pair_qubits_are_maximally_mixed() {
        let inv_sqrt2 = 0.5_f32.sqrt();
        let sim = SimulationState::from_statevector(
            2,
            vec![
                Complex {
                    re: inv_sqrt2,
                    im: 0.0,
                },
                Complex { re: 0.0, im: 0.0 },
                Complex { re: 0.0, im: 0.0 },
                Complex {
                    re: inv_sqrt2,
                    im: 0.0,
                },
            ],
        );
        assert_bloch(sim.bloch[0], 0.0, 0.0, 0.0);
        assert_bloch(sim.bloch[1], 0.0, 0.0, 0.0);
    }
}
