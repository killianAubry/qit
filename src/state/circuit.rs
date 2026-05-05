// Domain model for the user-authored circuit.
//
// A circuit is a `num_qubits × num_steps` grid where each cell may hold a
// single-gate placement. This is intentionally minimal — multi-qubit gates,
// classical wires, parameterized gates, etc. are out of scope for the UI
// prototype but the layout leaves room for them (e.g. a future
// `MultiGatePlacement { gate, qubits: Vec<usize>, step }`).

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GateKind {
    /// Plain boxed gate with a short label such as `H`, `RY`, `U`, `SW`.
    Box,
    /// Filled control dot used by controlled multi-qubit gates.
    Control,
    /// Measurement.
    Measure,
}

impl GateKind {
    pub fn label(self) -> &'static str {
        match self {
            GateKind::Box => "?",
            GateKind::Control => "•",
            GateKind::Measure => "M",
        }
    }
}

#[derive(Clone, Debug)]
pub struct GatePlacement {
    pub kind: GateKind,
    pub qubit: usize,
    pub step: usize,
    /// Short gate label shown inside the box. Ignored for controls.
    pub label: String,
    /// Shared id for multi-qubit visuals that should be connected by a
    /// vertical line in the renderer.
    pub link: Option<usize>,
}

impl GatePlacement {
    pub fn display_label(&self) -> &str {
        match self.kind {
            GateKind::Box => &self.label,
            GateKind::Control | GateKind::Measure => self.kind.label(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Circuit {
    pub num_qubits: usize,
    pub num_steps: usize,
    pub gates: Vec<GatePlacement>,
}

impl Circuit {
    pub fn new(num_qubits: usize, num_steps: usize) -> Self {
        Self { num_qubits, num_steps, gates: Vec::new() }
    }

    /// Place `kind` on `(qubit, step)`. Any existing gate at that cell is
    /// replaced. Step growth is automatic; out-of-range qubits are ignored.
    #[allow(dead_code)]
    pub fn place(&mut self, kind: GateKind, qubit: usize, step: usize) {
        self.place_gate(GatePlacement {
            kind,
            qubit,
            step,
            label: kind.label().to_string(),
            link: None,
        });
    }

    pub fn place_box(&mut self, label: impl Into<String>, qubit: usize, step: usize) {
        self.place_gate(GatePlacement {
            kind: GateKind::Box,
            qubit,
            step,
            label: label.into(),
            link: None,
        });
    }

    pub fn place_linked_box(
        &mut self,
        label: impl Into<String>,
        qubit: usize,
        step: usize,
        link: usize,
    ) {
        self.place_gate(GatePlacement {
            kind: GateKind::Box,
            qubit,
            step,
            label: label.into(),
            link: Some(link),
        });
    }

    pub fn place_control(&mut self, qubit: usize, step: usize, link: usize) {
        self.place_gate(GatePlacement {
            kind: GateKind::Control,
            qubit,
            step,
            label: String::new(),
            link: Some(link),
        });
    }

    pub fn place_measure(&mut self, qubit: usize, step: usize) {
        self.place_gate(GatePlacement {
            kind: GateKind::Measure,
            qubit,
            step,
            label: "M".to_string(),
            link: None,
        });
    }

    pub fn place_gate(&mut self, gate: GatePlacement) {
        if gate.qubit >= self.num_qubits {
            return;
        }
        if gate.step >= self.num_steps {
            self.num_steps = gate.step + 1;
        }
        self.gates.retain(|g| !(g.qubit == gate.qubit && g.step == gate.step));
        self.gates.push(gate);
    }

    // The methods below aren't used by today's read-only visualizer, but
    // they're part of the published `Circuit` API — a real simulator engine
    // or a future interactive editor will want them.

    #[allow(dead_code)]
    pub fn remove_at(&mut self, qubit: usize, step: usize) {
        self.gates.retain(|g| !(g.qubit == qubit && g.step == step));
    }

    #[allow(dead_code)]
    pub fn at(&self, qubit: usize, step: usize) -> Option<&GatePlacement> {
        self.gates.iter().find(|g| g.qubit == qubit && g.step == step)
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.gates.clear();
    }
}
