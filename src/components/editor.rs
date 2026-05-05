// Text editor — circuit source. Custom layouter matches `dsl::lex` tokens.

use std::sync::Arc;

use egui::text::{LayoutJob, TextFormat};
use egui::{Align2, FontId, Frame, Galley, Margin, RichText, TextEdit};

use crate::dsl::{python_tokens_as_editor_tokens, qasm_tokens_as_editor_tokens, TokenKind};
use crate::state::{AppState, SourceKind};
use crate::theme::{color, space};

const EMPTY_EDITOR_HINT: &str = "Open File:   ⌘O";

/// OpenQASM-style highlighting: simulator mode, `.qasm` save path, or buffer
/// shape (so QASM pasted while `qiskit` is selected still colors correctly).
fn use_openqasm_highlight(state: &AppState) -> bool {
    if state.simulator.source_kind() == SourceKind::OpenQasm {
        return true;
    }
    if state
        .circuit_file_path()
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("qasm"))
    {
        return true;
    }
    buffer_looks_like_openqasm(&state.editor_text)
}

/// Use a few structural hints instead of requiring a formal `OPENQASM` header.
/// That keeps syntax coloring active for partial / headerless `.qasm` snippets
/// while still rejecting normal Python/Qiskit files.
fn buffer_looks_like_openqasm(text: &str) -> bool {
    let mut seen_python = false;
    for raw in text.lines().take(64) {
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("//") || t.starts_with('#') {
            continue;
        }
        if looks_like_python(t) {
            seen_python = true;
            continue;
        }
        if looks_like_openqasm_line(t) {
            return true;
        }
    }
    !seen_python && text.contains("q[")
}

fn looks_like_python(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with("from ")
        || lower.starts_with("import ")
        || lower.starts_with("def ")
        || lower.starts_with("class ")
        || line.contains("QuantumCircuit(")
        || line.contains("qc.")
        || line.contains("circuit.")
}

fn looks_like_openqasm_line(line: &str) -> bool {
    let upper = line.to_ascii_uppercase();
    if upper.starts_with("OPENQASM")
        || upper.starts_with("INCLUDE")
        || upper.starts_with("QREG")
        || upper.starts_with("CREG")
        || upper.starts_with("MEASURE")
        || upper.starts_with("BARRIER")
        || line.contains("->")
    {
        return true;
    }

    if line.contains("q[") {
        let head = line
            .split_whitespace()
            .next()
            .unwrap_or("")
            .split('(')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        return !head.is_empty() && head.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    }

    false
}

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    let use_qasm = use_openqasm_highlight(state);
    let mut layouter = move |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| -> Arc<Galley> {
        let mut job = LayoutJob::default();
        let text = buf.as_str();
        for line in text.split_inclusive('\n') {
            let body = line.trim_end_matches('\n');
            let toks: Vec<_> = if use_qasm {
                qasm_tokens_as_editor_tokens(body)
            } else {
                python_tokens_as_editor_tokens(body)
            };
            for tok in toks {
                let c = token_color(tok.kind);
                job.append(
                    tok.text,
                    0.0,
                    TextFormat {
                        font_id: FontId::monospace(13.0),
                        color: c,
                        ..Default::default()
                    },
                );
            }
            if line.ends_with('\n') {
                job.append(
                    "\n",
                    0.0,
                    TextFormat {
                        font_id: FontId::monospace(13.0),
                        color: color::TEXT_PRIMARY,
                        ..Default::default()
                    },
                );
            }
        }
        job.wrap.max_width = wrap_width;
        ui.fonts_mut(|f| f.layout_job(job))
    };

    let diag_h = if state.diagnostics.is_empty() {
        0.0
    } else {
        12.0 + 14.0 * state.diagnostics.len() as f32
    };
    let editor_h = (ui.available_height() - diag_h - space::SM).max(80.0);
    let inner_min_h = (editor_h - 2.0 * space::SM).max(80.0);

    egui::ScrollArea::both()
        .id_salt("editor_scroll")
        .auto_shrink([false, false])
        .max_height(editor_h)
        .show(ui, |ui| {
            let editor_frame = Frame::NONE
                .fill(color::BG)
                .inner_margin(Margin::same(space::SM as i8));

            editor_frame.show(ui, |ui| {
                let fill_w = ui.available_width().max(10.0);
                let out = TextEdit::multiline(&mut state.editor_text)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(20)
                    .min_size(egui::vec2(fill_w, inner_min_h))
                    .desired_width(f32::INFINITY)
                    .lock_focus(true)
                    .frame(false)
                    .layouter(&mut layouter)
                    .show(ui);

                if state.editor_text.is_empty() {
                    ui.painter().text(
                        out.response.rect.center(),
                        Align2::CENTER_CENTER,
                        EMPTY_EDITOR_HINT,
                        FontId::monospace(13.0),
                        color::TEXT_DIM,
                    );
                }
            });
        });

    if !state.diagnostics.is_empty() {
        ui.add_space(space::XS);
        thin_rule(ui);
        ui.add_space(space::XS);
        for diag in &state.diagnostics {
            ui.label(
                RichText::new(format!("L{:<3} {}", diag.line + 1, diag.message))
                    .color(color::ACCENT_RED)
                    .monospace()
                    .size(11.0),
            );
        }
    }
}

fn token_color(kind: TokenKind) -> egui::Color32 {
    match kind {
        TokenKind::Comment => color::TEXT_DIM,
        TokenKind::Keyword => color::ACCENT_YELLOW,
        TokenKind::Qubit => color::ACCENT_PURPLE,
        TokenKind::Number => color::ACCENT_RED,
        TokenKind::Identifier => color::TEXT_PRIMARY,
        TokenKind::Whitespace => color::TEXT_PRIMARY,
        TokenKind::Punctuation => color::TEXT_MUTED,
        TokenKind::StringLit => color::ACCENT_GREEN,
    }
}

fn thin_rule(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 1.0),
        egui::Sense::hover(),
    );
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        egui::Stroke::new(1.0, color::GRID_LINE),
    );
}

#[cfg(test)]
mod tests {
    use super::buffer_looks_like_openqasm;

    #[test]
    fn detects_headerless_qasm_snippet() {
        let src = "// comment\nh q[0];\ncz q[0], q[1];\n";
        assert!(buffer_looks_like_openqasm(src));
    }

    #[test]
    fn rejects_qiskit_python() {
        let src = "from qiskit import QuantumCircuit\nqc = QuantumCircuit(2)\nqc.h(0)\n";
        assert!(!buffer_looks_like_openqasm(src));
    }
}
