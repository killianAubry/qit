// Composition root for application state.
//
//     editor_text  ──[parse]──▶  circuit          (live, on every edit)
//     editor_text  ──[run]────▶  simulation       (on ⌘R / :run)
//
// Three simulators are wired up:
//
//   * `SimulatorKind::OpenQasm`   buffer is OpenQASM 2 → Qiskit
//   * `SimulatorKind::Qiskit`     buffer is Python      → Qiskit (exec)
//   * `SimulatorKind::TurboSpin`  buffer is OpenQASM 2 → Spinoza CLI
//
// The visualizer-only `Circuit` is parsed in pure Rust here so the gate grid
// updates as you type. The actual statevector comes from the chosen runner;
// `SimulationState::from_statevector` derives probabilities and Bloch vectors
// from it, so every panel besides the editor and circuit visualizer reads off
// the same `statevector` source of truth.

pub mod circuit;
pub mod simulation;
pub mod simulator;
pub mod ui_state;

use std::path::PathBuf;

use crate::dsl::{parse_python, parse_qasm, Diagnostic};
use crate::tiling::{Tile, TileId, ViewKind};

pub use circuit::Circuit;
pub use simulation::SimulationState;
pub use simulator::{SimulatorKind, SourceKind, TurboSpinCompression};
pub use ui_state::{StatusKind, UiState};

pub struct AppState {
    pub editor_text: String,
    last_synced_text: String,
    /// When this differs from `simulator`, `ensure_synced` re-derives the
    /// derived state (qubit count, gates, diagnostics).
    last_sync_simulator: SimulatorKind,

    pub circuit: Circuit,
    /// Last OpenQASM parse that produced **zero** diagnostics — the grid
    /// stays on this until the buffer is error-free again.
    circuit_last_clean: Circuit,
    pub diagnostics: Vec<Diagnostic>,
    pub simulation: SimulationState,
    pub simulator: SimulatorKind,
    pub turbospin_compression: TurboSpinCompression,

    pub workspace_dir: PathBuf,

    pub tiles: Tile,
    pub focused_tile: TileId,

    pub ui: UiState,
}

impl AppState {
    pub fn new() -> Self {
        let simulator = SimulatorKind::OpenQasm;
        let editor_text = String::new();

        let (circuit, diagnostics) = parse_for(simulator, &editor_text);
        let circuit_last_clean = circuit.clone();
        let simulation = SimulationState::ground_state(circuit.num_qubits);

        let tiles = Tile::leaf(ViewKind::Editor);
        let focused_tile = tiles.first_leaf().id;

        let workspace_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        Self {
            last_synced_text: editor_text.clone(),
            last_sync_simulator: simulator,
            editor_text,
            circuit,
            circuit_last_clean,
            diagnostics,
            simulation,
            simulator,
            turbospin_compression: TurboSpinCompression::default(),
            workspace_dir,
            tiles,
            focused_tile,
            ui: UiState::default(),
        }
    }

    #[inline]
    pub fn circuit_file_path(&self) -> PathBuf {
        self.workspace_dir
            .join(format!("circuit.{}", self.simulator.circuit_extension()))
    }

    pub fn save_circuit_file(&mut self) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = self;
            return Err("save: use a desktop build".into());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = self.circuit_file_path();
            std::fs::write(&path, self.editor_text.as_bytes())
                .map_err(|e| format!("save {}: {e}", path.display()))?;
            Ok(())
        }
    }

    pub fn load_circuit_file(&mut self) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = self;
            return Err("load: use a desktop build".into());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = self.circuit_file_path();
            let s = std::fs::read_to_string(&path)
                .map_err(|e| format!("load {}: {e}", path.display()))?;
            self.load_editor_text_from_path(path, s)
        }
    }

    /// Replace the editor with file contents, set `workspace_dir` to the parent, sync, and rerun.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_editor_text_from_path(&mut self, path: PathBuf, text: String) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            self.workspace_dir = parent.to_path_buf();
        }
        self.editor_text = text;
        self.ensure_synced();
        self.rerun()
    }

    /// Cheap, runs every frame. Re-parses the buffer into `circuit` whenever
    /// the editor text or simulator selection has changed, so the visualizer
    /// stays in lockstep with the editor. Does **not** touch `simulation` —
    /// that is reserved for `rerun`.
    pub fn ensure_synced(&mut self) {
        let simulator_changed = self.simulator != self.last_sync_simulator;
        let text_changed = self.editor_text != self.last_synced_text;
        if !simulator_changed && !text_changed {
            return;
        }
        self.last_sync_simulator = self.simulator;
        self.last_synced_text = self.editor_text.clone();

        let (parsed, diagnostics) = parse_for(self.simulator, &self.editor_text);
        apply_circuit_parse(self.simulator, parsed, diagnostics, &mut self.circuit, &mut self.circuit_last_clean, &mut self.diagnostics);
    }

    /// Run the editor source through the selected simulator. On success
    /// replaces `self.simulation` so every statevector-derived panel
    /// (probability, state vector, Bloch) refreshes on the next frame.
    pub fn rerun(&mut self) -> Result<(), String> {
        let start_time = std::time::Instant::now();
        self.last_sync_simulator = self.simulator;
        self.last_synced_text = self.editor_text.clone();

        let (parsed, diagnostics) = parse_for(self.simulator, &self.editor_text);
        apply_circuit_parse(
            self.simulator,
            parsed,
            diagnostics,
            &mut self.circuit,
            &mut self.circuit_last_clean,
            &mut self.diagnostics,
        );

        let mut sim = run_simulator(
            self.simulator,
            &self.editor_text,
            self.turbospin_compression,
        )?;
        
        // Calculate theoretical memory overhead for the state vector
        // Simulator internal (Spinoza/Qiskit usually use f64 complex = 16 bytes)
        // + UI statevector (f32 complex = 8 bytes)
        // + UI probabilities (f32 = 4 bytes)
        // Total = 28 bytes per amplitude
        let bytes_per_amp = 28usize;
        let bytes = (1usize << sim.num_qubits) * bytes_per_amp;
        
        // Add baseline overhead (Python/CLI startup, basic structures)
        let baseline_bytes = if self.simulator == SimulatorKind::Qiskit {
            50_000_000 // Python/Qiskit has ~50MB baseline
        } else {
            2_000_000 // Spinoza Rust CLI is very lightweight, ~2MB baseline
        };

        let memory_mb = (bytes + baseline_bytes) as f64 / 1_048_576.0;
        
        sim.run_time_ms = Some(start_time.elapsed().as_secs_f64() * 1000.0);
        sim.memory_mb = Some(memory_mb);
        self.simulation = sim;
        Ok(())
    }

    /// Replace the editor with the current simulator's starter template.
    pub fn load_default_template(&mut self) {
        self.editor_text = self.simulator.default_template().to_string();
        self.ensure_synced();
    }
}

/// Apply parse output to `circuit` / `circuit_last_clean`.
///
/// OpenQASM (openqasm + turbospin): the grid advances only when there are no
/// diagnostics — otherwise we keep showing the last error-free layout.
/// Python: always follow the live parse (best-effort).
fn apply_circuit_parse(
    simulator: SimulatorKind,
    parsed: Circuit,
    diagnostics: Vec<Diagnostic>,
    circuit: &mut Circuit,
    last_clean: &mut Circuit,
    diags_out: &mut Vec<Diagnostic>,
) {
    *diags_out = diagnostics;
    if simulator.source_kind() == SourceKind::OpenQasm {
        if diags_out.is_empty() {
            *last_clean = parsed.clone();
            *circuit = parsed;
        } else {
            *circuit = last_clean.clone();
        }
    } else {
        *last_clean = parsed.clone();
        *circuit = parsed;
    }
}

/// Pure-Rust source → `Circuit` for the visualizer. Picks the parser based
/// on the simulator's source family.
fn parse_for(kind: SimulatorKind, src: &str) -> (Circuit, Vec<Diagnostic>) {
    match kind.source_kind() {
        SourceKind::OpenQasm => parse_qasm(src),
        SourceKind::Python => parse_python(src),
    }
}

fn run_simulator(
    kind: SimulatorKind,
    src: &str,
    turbospin_compression: TurboSpinCompression,
) -> Result<SimulationState, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (kind, src, turbospin_compression);
        return Err("run: desktop builds only".into());
    }

    #[cfg(not(target_arch = "wasm32"))]
    match kind {
        SimulatorKind::OpenQasm | SimulatorKind::Qiskit => {
            crate::qiskit::run_circuit_source(kind, src)
        }
        SimulatorKind::TurboSpin => {
            crate::turbospin::run_qasm_source(src, turbospin_compression)
        }
    }
}
