// Visualizer-only Qiskit/Python circuit parser.
//
// Best-effort preview parser. The actual simulation still runs through
// Python + Qiskit; this only keeps the read-only circuit view roughly aligned
// with the editor buffer.

use super::Diagnostic;
use crate::state::circuit::Circuit;

const MAX_QUBITS: usize = 20;

pub fn parse_python(src: &str) -> (Circuit, Vec<Diagnostic>) {
    let num_qubits = scan_quantum_circuit_size(src).clamp(1, MAX_QUBITS);

    let mut next_step = vec![0usize; num_qubits];
    let mut next_link = 0usize;
    let mut circuit = Circuit::new(num_qubits, 1);
    let mut diags = Vec::new();

    for (line_no, raw) in src.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line.is_empty() || !line.contains('.') || !line.contains('(') {
            continue;
        }

        let Some(dot) = line.find('.') else { continue };
        let after_dot = &line[dot + 1..];
        let Some(paren) = after_dot.find('(') else {
            continue;
        };
        let gate_name = after_dot[..paren].trim();

        let after_paren = &after_dot[paren + 1..];
        let Some(close) = find_matching_paren(after_paren) else {
            continue;
        };
        let args_src = &after_paren[..close];
        let groups = parse_qubit_groups(args_src, num_qubits);
        if groups.is_empty() {
            continue;
        }

        if let Err(message) = place_operation(
            &mut circuit,
            &mut next_step,
            &mut next_link,
            gate_name,
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
    gate_name: &str,
    groups: &[Vec<usize>],
) -> Result<(), String> {
    let lower = gate_name.to_ascii_lowercase();

    if lower == "measure" {
        place_measure_group(circuit, next_step, &groups[0]);
        return Ok(());
    }

    if let Some(shape) = controlled_shape(&lower) {
        let expected = shape.controls + shape.targets;
        if groups.len() != expected {
            return Err(format!(
                "`{gate_name}` expects {expected} qubit argument{}",
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
    } else {
        for lane in expand_parallel_lanes(groups) {
            place_linked_group(circuit, next_step, &[], &lane, &label, next_link);
        }
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
    let step = involved.iter().map(|&q| next_step[q]).max().unwrap_or(0);
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

fn parse_qubit_groups(args: &str, num_qubits: usize) -> Vec<Vec<usize>> {
    split_top_level_args(args)
        .into_iter()
        .filter_map(|arg| {
            let qs = extract_ints(arg, num_qubits);
            (!qs.is_empty()).then_some(qs)
        })
        .collect()
}

fn split_top_level_args(args: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (i, c) in args.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                out.push(args[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    if start <= args.len() {
        out.push(args[start..].trim());
    }
    out.into_iter().filter(|s| !s.is_empty()).collect()
}

fn extract_ints(args: &str, num_qubits: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in args.chars() {
        if c.is_ascii_digit() {
            cur.push(c);
        } else if !cur.is_empty() {
            flush_int(&mut cur, &mut out, num_qubits);
        }
    }
    flush_int(&mut cur, &mut out, num_qubits);
    out
}

fn flush_int(cur: &mut String, out: &mut Vec<usize>, max: usize) {
    if cur.is_empty() {
        return;
    }
    if let Ok(n) = cur.parse::<usize>() {
        if n < max {
            out.push(n);
        }
    }
    cur.clear();
}

fn scan_quantum_circuit_size(src: &str) -> usize {
    const NEEDLE: &[u8] = b"QuantumCircuit(";
    let bytes = src.as_bytes();
    let mut best = 1usize;
    let mut i = 0;
    while i + NEEDLE.len() <= bytes.len() {
        if &bytes[i..i + NEEDLE.len()] == NEEDLE {
            i += NEEDLE.len();
            while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
                i += 1;
            }
            let start = i;
            while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                i += 1;
            }
            if start != i {
                if let Ok(n) = src[start..i].parse::<usize>() {
                    best = best.max(n);
                }
            }
        } else {
            i += 1;
        }
    }
    best
}

fn find_matching_paren(s: &str) -> Option<usize> {
    let mut depth = 1usize;
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

fn strip_comment(line: &str) -> &str {
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
