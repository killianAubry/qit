# qsim_ui

A Rust quantum-circuit playground built on
[`egui`](https://github.com/emilk/egui)/[`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe).

The main view is a `micro`-style **text editor** for circuit source. Auxiliary
views (circuit diagram, probability distribution, state vector, 3D Bloch
spheres) live in a **tiling window manager** — open them with `⌘T`, move
focus with `⌘H/J/K/L`, close with `⌘W`. There are no buttons; everything is a
keybind.

Three simulators ship in the box, all delivering the **same statevector
contract** — every panel besides the editor and circuit visualizer reads off
`state.simulation.statevector`; probabilities and Bloch vectors are derived
in pure Rust.

| Mode        | Editor language  | Files          | Runner |
| ----------- | ---------------- | -------------- | ------ |
| `openqasm`  | OpenQASM 2.0     | `circuit.qasm` | `python3 scripts/qiskit_run.py qasm <file>` |
| `qiskit`    | Python (Qiskit)  | `circuit.py`   | `python3 scripts/qiskit_run.py py <file>`   |
| `turbospin` | OpenQASM 2.0     | `circuit.qasm` | `cargo run -p spinoza -- --qasm <file> --no-compression` or `--compression-bits <1..8>` (in `TurboSpin/`) |

## Keybinds

| Action | macOS |
| ------ | ----- |
| **Command Palette** | `⌘E` |
| **Open Tile Picker** | `⌘T` |
| **Open Specific Tile** | `⌘1` .. `⌘6` |
| **Run Simulation** | `⌘R` |
| **Open File** | `⌘O` |
| **Save File** | `⌘S` |
| **Close Tile** | `⌘W` |
| **Move Focus** | `⌘H/J/K/L` |
| **Cycle Focus** | `⌘Tab` / `⌘⇧Tab` |
| **Split View** | `⌘⇧H/J/K/L` |

## Available Views

*   **Editor**: Main text editor for writing quantum circuits.
*   **Circuit**: Visual representation of the circuit grid.
*   **Probability**: Bar chart of basis state probabilities.
*   **State Vector**: Amplitudes of the final state vector.
*   **Bloch 3D**: Interactive 3D Bloch spheres for each qubit.
*   **Benchmarks**: Displays simulation runtime and state vector memory overhead.

Switch with the dropdown in the title bar (or `:sim openqasm | qiskit |
turbospin`). When the editor is empty or still holds the previous mode's
starter template, switching auto-loads the new template; otherwise your
edits are preserved.

When `turbospin` is selected, a second title-bar dropdown appears for
TurboSpin compression: `exact` keeps the original statevector, while `1..8`
bits compress then immediately decompress before the UI renders the result.

The **circuit visualizer** parses the editor source on every keystroke
(OpenQASM and Python both, best-effort) so the gate grid follows live
edits. The simulation panels only refresh when you press `⌘R`.

## Run

```bash
cargo run --release

# OpenQASM / Qiskit modes need Python — install once:
python3 -m pip install qiskit
# (override interpreter via $QISKIT_PYTHON if needed)

# TurboSpin mode shells out to the bundled Spinoza workspace at
# TurboSpin/Cargo.toml. It's pinned to nightly via its own
# rust-toolchain.toml; first run will download the toolchain.
#   $TURBOSPIN_MANIFEST  override path to TurboSpin/Cargo.toml
#   $TURBOSPIN_CARGO     override `cargo` invocation (e.g. `rustup run nightly cargo`)
```

> First build downloads `egui`/`eframe`; subsequent runs are incremental.

## Keybinds

| Key                  | Action                                                  |
| -------------------- | ------------------------------------------------------- |
| `⌘E`                 | command palette                                         |
| `⌘T`                 | tile picker (open a new tile next to the focused one)   |
| `⌘W`                 | close focused tile                                      |
| `⌘R`                 | run the editor source through Qiskit                    |
| `⌘S` / `⌘O`          | save / load `circuit.<ext>` to/from the workspace folder|
| `⌘H` `⌘J` `⌘K` `⌘L`  | focus tile to the left / down / up / right             |
| `⌘Tab` / `⌘⇧Tab`     | cycle tile focus forward / backward (layout order)      |
| `⌘1 … ⌘5`            | open circuit / probabilities / SV / bloch / editor tile |
| `Esc`                | dismiss palette / picker                                |

The folder glyph in the top-left opens a native folder picker — that
directory is where `⌘S` / `⌘O` read and write `circuit.<ext>`. Drag a tile
border to resize.

**Syntax highlighting:** OpenQASM rules apply when the simulator is `openqasm`
or `turbospin`, when the workspace save path is `circuit.qasm`, or when the
first substantive line of the buffer looks like OpenQASM (`OPENQASM`, `qreg`,
`include`). Otherwise the editor uses the Python highlighter for `qiskit` mode.

## Editor sources

### OpenQASM 2.0 (`circuit.qasm`)

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

### Qiskit (`circuit.py`)

```python
from qiskit import QuantumCircuit

qc = QuantumCircuit(2, 2)
qc.h(0)
qc.cx(0, 1)
qc.measure([0, 1], [0, 1])
```

In Qiskit mode the runner exec's the file in a fresh namespace and looks for
a `QuantumCircuit` named `qc` (preferred) or `circuit`. It falls back to the
last `QuantumCircuit` defined in the module if neither name is present.

Final `measure` / `barrier` instructions are stripped before evolving the
state vector so the probability / amplitude / Bloch panels reflect the
*pre-measurement* state.

### Syntax highlighting

| token         | color   |
| ------------- | ------- |
| keyword       | yellow  |
| gate / class  | purple  |
| number        | red     |
| string        | green   |
| comment       | dim     |
| punctuation   | muted   |

## Command palette (`⌘E`)

```
:sim <name>          openqasm | qiskit | turbospin
:open <view>         circuit | prob | sv | bloch | editor
:save                write circuit.<ext> to the workspace folder
:load                read  circuit.<ext> from the workspace folder
:close               close the focused tile
:run                 run the editor source through the selected simulator (also ⌘R)
:compress <exact|1..8> turbospin compression setting
:reset               replace the buffer with the current mode's template
:clear               empty the editor
:help                hint reminder
```

## Layout & architecture

```
src/
├── main.rs                       entry point
├── app.rs                        eframe::App: keybinds + tile rendering
├── theme.rs                      design tokens + style installer
├── grid.rs                       reusable cell-grid + snap utility
├── tiling.rs                     tile tree (Leaf / Split) + drag/focus ops
├── workspace.rs                  native folder picker (rfd) + wasm stub
├── qiskit.rs                     Python runner dispatch + JSON parsing
├── turbospin.rs                  Spinoza CLI dispatch + statevector parsing
├── dsl/
│   ├── mod.rs
│   ├── lex.rs                    Token / TokenKind shared by all lexers
│   ├── qasm_lex.rs               OpenQASM 2 highlighter
│   ├── qasm_parse.rs             OpenQASM → Circuit (visualizer)
│   ├── python_lex.rs             Python highlighter
│   └── python_parse.rs           Python → Circuit (visualizer)
├── state/
│   ├── mod.rs                    AppState (composition root)
│   ├── circuit.rs                gate / qubit / step model (visualizer-only)
│   ├── simulation.rs             statevector + derived probs/Bloch
│   ├── simulator.rs              SimulatorKind + per-mode templates
│   └── ui_state.rs               modal flags, status flash
├── scripts/
│   └── qiskit_run.py             OpenQASM | Python → {num_qubits, statevector}
└── components/
    ├── mod.rs                    `render_view` dispatch + module exports
    ├── editor.rs                 syntax-highlighted text editor (main view)
    ├── circuit_visualizer.rs     read-only grid renderer
    ├── probability_panel.rs      bar viz, scrollable
    ├── state_vector.rs           amplitude list, monospace
    ├── bloch_3d.rs               3D Bloch sphere with mouse-drag rotation
    ├── status_bar.rs             top + bottom status strips
    ├── command_palette.rs        `:`-style overlay (⌘E)
    └── tile_picker.rs            view chooser overlay (⌘T)
```

### Design rules

- **No buttons.** Every action is a keybind or a `:`-command.
- **Editor first.** Source flows one-way: `editor_text → simulator
  → SimulationState`. Visual panels never mutate state.
- **Statevector is the source of truth.** Each runner only has to produce
  `(num_qubits, Vec<Complex>)`; `SimulationState::from_statevector` derives
  probabilities and per-qubit Bloch vectors in pure Rust, so the SV /
  probability / Bloch panels render identically across every backend.
- **Run is explicit.** Editing re-parses the circuit visualizer live, but
  the simulation panels refresh exclusively on `⌘R` (or `:run`) — so a
  half-typed circuit doesn't fire a runner every keystroke.
- **Trait-free dispatch.** Each view is a free `show` function;
  `components::render_view(view, ui, &mut state, &mut leaf)` picks the right
  one based on `ViewKind`. Adding a view is one match arm + one module.
- **Per-tile state on the leaf.** `LeafTile` carries its own `bloch_yaw` /
  `bloch_pitch`, so two Bloch tiles can show different angles.
- **Thin borders only.** Tile boundaries use a 1-pixel stroke (`GRID_LINE`,
  or `ACCENT_YELLOW` when focused). Splits are 1-pixel handles.

### 3D Bloch sphere

Software wireframe — three great circles (equator + two meridians) sampled at
64 points each, rotated by the tile's yaw/pitch, projected orthographically.
Depth-based alpha makes the back half read as occluded. Mouse-drag inside any
sphere updates the tile-wide rotation, so all spheres in a tile rotate
together. The Bloch vector itself is drawn last with the same depth shading.

### Adding a simulator

Every simulator only needs to produce `(num_qubits, Vec<Complex>)` — derived
panels follow automatically. Two contracts already plug into that:

`scripts/qiskit_run.py <mode> <file>` (mode = `qasm` | `py`):

```json
{ "num_qubits": 2, "statevector": [{"re": 0.707, "im": 0}, ...] }
```

`cargo run -p spinoza -- --qasm <file> --no-compression` (TurboSpin) — plain
text on stdout in the form
`<index> | <bin> | re=<f> | im=<f> | magnitude=<f> | probability=<f>`,
preceded by a `qubits:` header.

Adding another backend is one match arm in `state::run_simulator` plus a
`run_<name>_source` function returning `Result<SimulationState, String>`
(use `SimulationState::from_statevector` to build the result).
