# qit

**qit** is a desktop **quantum circuit workbench** for editing OpenQASM-style programs, running them through **multiple backends**, and **benchmarking** simulators side by side. Built with [`egui`](https://github.com/emilk/egui) / [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe) in Rust.

Repository: [github.com/killianAubry/qit](https://github.com/killianAubry/qit)

The high-performance **TurboSpin** backend (Spinoza + optional BACQS/RPDQ compression) is tracked as a **git submodule** from [github.com/killianAubry/TurboSpin](https://github.com/killianAubry/TurboSpin). If your checkout of **qit** uses a submodule for `TurboSpin/`, clone with:

```bash
git clone --recurse-submodules https://github.com/killianAubry/qit.git
cd qit
```

If you cloned without submodules:

```bash
git submodule update --init --recursive
```

If `TurboSpin/` is a normal directory in your tree (no submodule), develop as usual and point `TURBOSPIN_MANIFEST` at `TurboSpin/Cargo.toml` if the layout differs.

---

## Simulation benchmarking

qit is built to **measure and compare** simulators on the **same circuit** under a **shared statevector contract**:

- **Compare mode** — Choose a primary simulator and an optional **compare** simulator (configuration popup **⌘,** or `:compare`). After **Run** (`⌘R`), views such as probability, Bloch, state vector, **fidelity**, **entanglement**, and **density matrix** can **overlay** two runs for direct comparison.
- **Independent TurboSpin settings** — Use `:compress` / `:compare_compress` for bit depth and `:tsmode` / `:compare_tsmode` for **`bacqs`** vs **`rpdq`** on primary vs compare paths.
- **Runtime** — Successful runs report **elapsed time** on the bottom status line next to **ok**.
- **Metrics** — `MetricsTracker` / `state/metrics.rs` records timing and optional compression metadata; the **Noise** panel surfaces **runtime**, **memory-oriented** bars, and a compact **quality** summary next to noise/device controls.

Together, this supports benchmarking **accuracy, wall-clock time, and resource signals** across **Qiskit** (Python + `scripts/qiskit_run.py`) and **TurboSpin** (native `spinoza` CLI).

---

## Simulators

| Mode | Editor / file | Runner |
| ---- | ---------------- | ------ |
| `qiskit` | OpenQASM · `circuit.qasm` | `python3 scripts/qiskit_run.py qasm <file>` · set **`QISKIT_PYTHON`** for a venv that has **Qiskit**. |
| `turbospin` | OpenQASM · `circuit.qasm` | Spinoza in `TurboSpin/`: **`--qasm`**, **`--comp-bit` 0–8**, optional **`--compression-mode`** `bacqs` \| `rpdq`. |

Full CLI and OpenQASM details: [TurboSpin/README.md](https://github.com/killianAubry/TurboSpin/blob/main/README.md).

<img width="1470" height="923" alt="qit UI" src="https://github.com/user-attachments/assets/fdcfbe22-a651-421c-86cb-bba0f4519449" />

---

## Keybinds

| Action | Binding |
| ------ | ------- |
| Command palette | **⌘E** |
| Tile picker | **⌘T** |
| Tiles **⌘1** … **⌘6** | Common views |
| Run | **⌘R** |
| Open / save circuit | **⌘O** / **⌘S** |
| Close tile | **⌘W** |
| Move focus | **⌘H** / **J** / **K** / **L** |
| Cycle focus | **⌘Tab** / **⌘⇧Tab**, also **Ctrl+Tab** / **Ctrl+⇧Tab** |
| Split | **⌘⇧H/J/K/L** |
| Configuration | **⌘,** |

On Linux and Windows, **⌘** is **Ctrl**.

---

## Views

Open from **⌘T** or **`:open`**.

| View | Role |
| ---- | ---- |
| **Editor** | Circuit source |
| **Circuit** | Live gate diagram from parser |
| **Probability** | Basis probabilities |
| **State vector** | Amplitudes |
| **Bloch 3D** | Per-qubit Bloch |
| **Noise** | Noise / device tuning + benchmark-style summaries |
| **Fidelity** | Compare overlap when two runs exist |
| **Entanglement** | Pairwise entanglement |
| **Density matrix** | ρ (primary and compare when enabled) |

Examples: `:open fid`, `ent`, `dm`. Statevector-backed panels update on **Run**; the circuit view updates while editing.

---

## Run

```bash
cargo run --release

python3 -m pip install qiskit   # or a venv; export QISKIT_PYTHON=/path/to/python
git submodule update --init --recursive TurboSpin   # when using submodule layout

# TurboSpin follows TurboSpin/rust-toolchain.toml (often nightly).
# Optional: TURBOSPIN_MANIFEST, TURBOSPIN_CARGO
```

---

## Command palette (`⌘E`)

```
:sim <name>                 qiskit | turbospin
:compare <name|off>        second simulator
:compress / :compare_compress   turbospin bit depth
:tsmode / :compare_tsmode   bacqs | rpdq
:open <view>                circuit | prob | sv | bloch | editor | noise | fid | ent | dm | …
:run  :save  :load  :close  :config  :reset  :clear  :help
```

---

## Project layout

```
src/
├── app.rs
├── qiskit.rs
├── turbospin.rs
├── state/mod.rs
├── state/simulation.rs
├── state/metrics.rs
├── state/noise.rs
├── components/
│   ├── editor.rs
│   ├── circuit_visualizer.rs
│   ├── probability_panel.rs
│   ├── state_vector.rs
│   ├── bloch_3d.rs
│   ├── noise_panel.rs
│   ├── fidelity_panel.rs
│   ├── entanglement_panel.rs
│   ├── density_matrix_panel.rs
│   ├── command_palette.rs
│   ├── config_popup.rs
│   ├── status_bar.rs
│   └── tile_picker.rs
└── scripts/qiskit_run.py
```

---

## Editor source

Both selectable simulators use **OpenQASM 2.0** in **`circuit.qasm`** for the bundled flow. Example:

```qasm
OPENQASM 2.0;
include "qelib1.inc";

qreg q[2];
creg c[2];

h q[0];
cx q[0], q[1];

measure q[0] -> c[0];
measure q[1] -> c[1];
```

Final measurements are dropped for statevector evolution so panels show the **pre-measurement** state.

---

## Adding a simulator

Return `(num_qubits, statevector)`; use `SimulationState::from_statevector`. Wire a new `SimulatorKind` variant, `run_simulator` arm, and UI (config / status bar) as needed.

---

## License

See licenses in this tree and under `TurboSpin/` (e.g. `TurboSpin/LICENSE`).
