// Visualizer-only OpenQASM parser.
//
// Best-effort preview parser for the read-only circuit grid. The simulator
// backends still own the real semantics; this layer tries to keep the circuit
// view visually faithful to the current editor buffer by:
//   * tracking all declared `qreg`s (not just the first one),
//   * rendering arbitrary gate labels instead of a tiny fixed gate set,
//   * linking multi-qubit operations with connector ids for the renderer.

use super::Diagnostic;
use crate::state::circuit::Circuit;

const MAX_QUBITS: usize = 10;

pub fn parse_qasm(src: &str) -> (Circuit, Vec<Diagnostic>) {
    let registers = QRegisters::from_source(src);
    let num_qubits = registers.num_qubits();

    let mut diags = Vec::new();
    let mut next_step = vec![0usize; num_qubits];
    let mut next_link = 0usize;
    let mut circuit = Circuit::new(num_qubits, 1);

    for (line_no, raw) in src.lines().enumerate() {
        let mut line = strip_comment(raw).trim().trim_end_matches(';').trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = strip_if_guard(line) {
            line = rest;
        }
        if line.is_empty() || is_directive(line) {
            continue;
        }

        let (name, args) = split_name_and_args(line);
        if name.is_empty() {
            continue;
        }

        let qubit_part = if name.eq_ignore_ascii_case("measure") {
            args.split("->").next().unwrap_or(args)
        } else {
            args
        };
        let Some(groups) = parse_qubit_groups(qubit_part, &registers) else {
            diags.push(Diagnostic {
                line: line_no,
                message: format!("`{name}` — couldn't parse qubit args"),
            });
            continue;
        };
        if groups.is_empty() {
            diags.push(Diagnostic {
                line: line_no,
                message: format!("`{name}` — missing qubit args"),
            });
            continue;
        }

        if let Err(message) = place_operation(
            &mut circuit,
            &mut next_step,
            &mut next_link,
            name,
            &groups,
        ) {
            diags.push(Diagnostic {
                line: line_no,
                message,
            });
        }
    }

    let max_step = next_step.iter().copied().max().unwrap_or(0);
    circuit.num_steps = max_step.max(8);
    (circuit, diags)
}

fn place_operation(
    circuit: &mut Circuit,
    next_step: &mut [usize],
    next_link: &mut usize,
    name: &str,
    groups: &[Vec<usize>],
) -> Result<(), String> {
    let lower = name.to_ascii_lowercase();

    if lower == "measure" {
        if groups.len() != 1 {
            return Err(format!("`{name}` expects one quantum argument"));
        }
        place_measure_group(circuit, next_step, &groups[0]);
        return Ok(());
    }

    if let Some(shape) = controlled_shape(&lower) {
        let expected = shape.controls + shape.targets;
        if groups.len() != expected {
            return Err(format!(
                "`{name}` expects {expected} qubit argument{}",
                if expected == 1 { "" } else { "s" },
            ));
        }
        for lane in expand_parallel_lanes(groups) {
            let controls = &lane[..shape.controls];
            let targets = &lane[shape.controls..];
            place_linked_group(
                circuit,
                next_step,
                controls,
                targets,
                &shape.target_label,
                next_link,
            );
        }
        return Ok(());
    }

    if lower == "swap" {
        if groups.len() != 2 {
            return Err("`swap` expects two qubit arguments".into());
        }
        for lane in expand_parallel_lanes(groups) {
            place_linked_group(circuit, next_step, &[], &lane, "SW", next_link);
        }
        return Ok(());
    }

    let label = gate_label(&lower);
    if groups.len() == 1 {
        place_box_group(circuit, next_step, &label, &groups[0]);
        return Ok(());
    }

    for lane in expand_parallel_lanes(groups) {
        place_linked_group(circuit, next_step, &[], &lane, &label, next_link);
    }
    Ok(())
}

fn place_box_group(circuit: &mut Circuit, next_step: &mut [usize], label: &str, qubits: &[usize]) {
    let Some((step, involved)) = reserve_step(next_step, qubits) else {
        return;
    };
    for &q in &involved {
        circuit.place_box(label, q, step);
    }
    advance_step(next_step, &involved, step + 1);
}

fn place_measure_group(circuit: &mut Circuit, next_step: &mut [usize], qubits: &[usize]) {
    let Some((step, involved)) = reserve_step(next_step, qubits) else {
        return;
    };
    for &q in &involved {
        circuit.place_measure(q, step);
    }
    advance_step(next_step, &involved, step + 1);
}

fn place_linked_group(
    circuit: &mut Circuit,
    next_step: &mut [usize],
    controls: &[usize],
    targets: &[usize],
    target_label: &str,
    next_link: &mut usize,
) {
    let mut involved = Vec::with_capacity(controls.len() + targets.len());
    involved.extend_from_slice(controls);
    involved.extend_from_slice(targets);
    let Some((step, involved)) = reserve_step(next_step, &involved) else {
        return;
    };

    let link = *next_link;
    *next_link += 1;

    for &q in controls {
        if involved.contains(&q) {
            circuit.place_control(q, step, link);
        }
    }
    for &q in targets {
        if involved.contains(&q) {
            circuit.place_linked_box(target_label, q, step, link);
        }
    }
    advance_step(next_step, &involved, step + 1);
}

fn reserve_step(next_step: &[usize], qubits: &[usize]) -> Option<(usize, Vec<usize>)> {
    let mut involved = Vec::new();
    for &q in qubits {
        if q >= next_step.len() || involved.contains(&q) {
            continue;
        }
        involved.push(q);
    }
    if involved.is_empty() {
        return None;
    }
    let step = involved
        .iter()
        .map(|&q| next_step[q])
        .max()
        .unwrap_or(0);
    Some((step, involved))
}

fn advance_step(next_step: &mut [usize], qubits: &[usize], step: usize) {
    for &q in qubits {
        if q < next_step.len() {
            next_step[q] = step;
        }
    }
}

fn expand_parallel_lanes(groups: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let width = groups.iter().map(Vec::len).max().unwrap_or(0).max(1);
    let mut out = Vec::with_capacity(width);
    for lane in 0..width {
        let mut row = Vec::with_capacity(groups.len());
        for group in groups {
            let idx = if group.len() == 1 { 0 } else { lane.min(group.len() - 1) };
            if let Some(&q) = group.get(idx) {
                row.push(q);
            }
        }
        if !row.is_empty() {
            out.push(row);
        }
    }
    out
}

fn controlled_shape(name: &str) -> Option<ControlledShape> {
    let controls = name.chars().take_while(|&c| c == 'c').count();
    if controls == 0 {
        return None;
    }
    let tail = &name[controls..];
    if tail.is_empty() {
        return None;
    }
    if tail == "swap" {
        return Some(ControlledShape {
            controls,
            targets: 2,
            target_label: "SW".to_string(),
        });
    }
    Some(ControlledShape {
        controls,
        targets: 1,
        target_label: gate_label(tail),
    })
}

fn gate_label(name: &str) -> String {
    match name {
        "id" => "I".into(),
        "swap" => "SW".into(),
        "reset" => "RST".into(),
        "u" => "U".into(),
        "u1" => "U1".into(),
        "u2" => "U2".into(),
        "u3" => "U3".into(),
        _ => {
            let compact: String = name
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .map(|c| c.to_ascii_uppercase())
                .collect();
            if compact.is_empty() {
                return "?".into();
            }
            if compact.len() <= 3 {
                compact
            } else if compact.starts_with('U') && compact.len() >= 2 {
                compact[..2].to_string()
            } else {
                compact[..3].to_string()
            }
        }
    }
}

fn parse_qubit_groups(args: &str, registers: &QRegisters) -> Option<Vec<Vec<usize>>> {
    let mut out = Vec::new();
    for tok in args.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let expanded = registers.resolve(tok)?;
        if expanded.is_empty() {
            return None;
        }
        out.push(expanded);
    }
    Some(out)
}

fn split_name_and_args(line: &str) -> (&str, &str) {
    let bytes = line.as_bytes();
    let mut end = 0;
    while end < bytes.len() {
        let c = bytes[end] as char;
        if c.is_ascii_alphanumeric() || c == '_' {
            end += 1;
        } else {
            break;
        }
    }
    let name = &line[..end];
    let mut rest = line[end..].trim_start();
    if rest.starts_with('(') {
        if let Some(close) = find_matching_paren(rest) {
            rest = rest[close + 1..].trim_start();
        }
    }
    (name, rest)
}

fn find_matching_paren(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

fn strip_if_guard(line: &str) -> Option<&str> {
    let rest = strip_word_prefix(line, "if")?;
    let rest = rest.trim_start();
    if !rest.starts_with('(') {
        return None;
    }
    let close = find_matching_paren(rest)?;
    Some(rest[close + 1..].trim_start())
}

fn is_directive(line: &str) -> bool {
    line.starts_with("OPENQASM")
        || strip_word_prefix(line, "include").is_some()
        || strip_word_prefix(line, "qreg").is_some()
        || strip_word_prefix(line, "creg").is_some()
        || strip_word_prefix(line, "barrier").is_some()
        || strip_word_prefix(line, "gate").is_some()
        || strip_word_prefix(line, "opaque").is_some()
}

fn parse_register_decl(rest: &str) -> Option<(String, usize)> {
    let open = rest.find('[')?;
    let close = rest[open + 1..].find(']')?;
    let name = rest[..open].trim();
    if name.is_empty() {
        return None;
    }
    let size = rest[open + 1..open + 1 + close].trim().parse::<usize>().ok()?;
    Some((name.to_string(), size))
}

fn strip_word_prefix<'a>(line: &'a str, word: &str) -> Option<&'a str> {
    if line.len() < word.len() || !line[..word.len()].eq_ignore_ascii_case(word) {
        return None;
    }
    let stripped = &line[word.len()..];
    let next = stripped.chars().next();
    match next {
        None => Some(""),
        Some(c) if c.is_ascii_whitespace() || c == '[' || c == '(' => Some(stripped),
        _ => None,
    }
}

fn strip_comment(line: &str) -> &str {
    let line = if let Some(i) = line.find("//") { &line[..i] } else { line };
    if let Some(i) = line.find('#') {
        &line[..i]
    } else {
        line
    }
}

struct ControlledShape {
    controls: usize,
    targets: usize,
    target_label: String,
}

#[derive(Clone)]
struct QRegister {
    name: String,
    base: usize,
    size: usize,
}

struct QRegisters {
    regs: Vec<QRegister>,
    total_qubits: usize,
}

impl QRegisters {
    fn from_source(src: &str) -> Self {
        let mut regs = Vec::new();
        let mut total_qubits = 0usize;

        for raw in src.lines() {
            let line = strip_comment(raw).trim().trim_end_matches(';').trim();
            let Some(rest) = strip_word_prefix(line, "qreg") else {
                continue;
            };
            let Some((name, size)) = parse_register_decl(rest.trim()) else {
                continue;
            };
            if total_qubits >= MAX_QUBITS {
                continue;
            }
            let visible = size.min(MAX_QUBITS - total_qubits);
            if visible == 0 {
                continue;
            }
            regs.push(QRegister {
                name,
                base: total_qubits,
                size: visible,
            });
            total_qubits += visible;
        }

        Self {
            regs,
            total_qubits: total_qubits.max(1),
        }
    }

    fn num_qubits(&self) -> usize {
        self.total_qubits
    }

    fn resolve(&self, token: &str) -> Option<Vec<usize>> {
        let token = token.trim();
        if token.is_empty() {
            return None;
        }

        if let (Some(open), Some(close)) = (token.find('['), token.rfind(']')) {
            let name = token[..open].trim();
            let idx = token[open + 1..close].trim().parse::<usize>().ok()?;
            let reg = self.regs.iter().find(|r| r.name == name)?;
            return (idx < reg.size).then(|| vec![reg.base + idx]);
        }

        if let Some(reg) = self.regs.iter().find(|r| r.name == token) {
            return Some((0..reg.size).map(|i| reg.base + i).collect());
        }

        let trimmed = token.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == '_');
        let idx = trimmed.parse::<usize>().ok()?;
        (idx < self.total_qubits).then(|| vec![idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_rotation_and_controlled_gates() {
        let src = "OPENQASM 2.0;\nqreg q[4];\nry(0.1) q[0];\nrz(0.2) q[1];\ncp(pi/2) q[1], q[2];\nu(0,0,0) q[3];\n";
        let (circuit, diags) = parse_qasm(src);

        assert!(diags.is_empty());
        assert_eq!(circuit.num_qubits, 4);
        assert_eq!(circuit.gates.len(), 5);

        assert!(circuit.gates.iter().any(|g| g.kind == crate::state::circuit::GateKind::Box && g.label == "RY"));
        assert!(circuit.gates.iter().any(|g| g.kind == crate::state::circuit::GateKind::Box && g.label == "RZ"));
        assert!(circuit.gates.iter().any(|g| g.kind == crate::state::circuit::GateKind::Control));
        assert!(circuit.gates.iter().any(|g| g.kind == crate::state::circuit::GateKind::Box && g.label == "P"));
        assert!(circuit.gates.iter().any(|g| g.kind == crate::state::circuit::GateKind::Box && g.label == "U"));
    }

    #[test]
    fn sums_multiple_qregs_and_offsets_indices() {
        let src = "OPENQASM 2.0;\nqreg a[2];\nqreg b[2];\nh a[1];\ncx a[1], b[0];\n";
        let (circuit, diags) = parse_qasm(src);

        assert!(diags.is_empty());
        assert_eq!(circuit.num_qubits, 4);
        assert!(circuit.gates.iter().any(|g| g.qubit == 1 && g.label == "H"));
        assert!(circuit.gates.iter().any(|g| g.qubit == 1 && g.kind == crate::state::circuit::GateKind::Control));
        assert!(circuit.gates.iter().any(|g| g.qubit == 2 && g.label == "X"));
    }

    #[test]
    fn expands_register_wide_measurement() {
        let src = "OPENQASM 2.0;\nqreg q[3];\ncreg c[3];\nmeasure q -> c;\n";
        let (circuit, diags) = parse_qasm(src);

        assert!(diags.is_empty());
        assert_eq!(circuit.gates.len(), 3);
        assert!(circuit.gates.iter().all(|g| g.kind == crate::state::circuit::GateKind::Measure));
        assert!(circuit.gates.iter().all(|g| g.step == 0));
    }
}
