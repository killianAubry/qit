// Fidelity & error analysis panel.
//
// Shows state fidelity, trace distance, and per-basis-state probability
// comparison between the primary simulation and the compare baseline.
// Designed for quantum error research — compare noisy vs. ideal states.

use egui::{CornerRadius, RichText, Stroke, StrokeKind, Vec2};

use crate::state::simulation;
use crate::state::AppState;
use crate::theme::{color, space};

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    let sim = &state.simulation;
    let n = sim.num_qubits;

    if n == 0 || sim.statevector.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(space::XL);
            ui.label(RichText::new("no simulation data").color(color::TEXT_MUTED).monospace());
            ui.label(RichText::new("run ⌘R to compute metrics").color(color::TEXT_DIM).monospace().size(11.0));
        });
        return;
    }

    ui.vertical_centered(|ui| {
        ui.add_space(space::SM);
        ui.label(RichText::new("fidelity analysis").color(color::TEXT_MUTED).monospace().size(11.0));
        ui.add_space(space::SM);
    });

    hairline(ui);
    ui.add_space(space::MD);

    egui::ScrollArea::vertical()
        .id_salt("fidelity_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // ── Compare fidelity ──
            if let Some(ref cmp) = state.compare_simulation {
                let fid = simulation::state_fidelity(&sim.statevector, &cmp.statevector);
                let trace_dist = simulation::trace_distance(fid);

                section_header(ui, "primary vs compare");
                ui.add_space(space::SM);

                // Fidelity card
                metric_card(ui, "state fidelity", fid, color::ACCENT_GREEN, |v| format!("{:.6}", v));
                ui.add_space(space::SM);
                metric_card(ui, "trace distance", trace_dist, color::ACCENT_RED, |v| format!("{:.6}", v));
                ui.add_space(space::SM);
                metric_card(ui, "infidelity (1-F)", 1.0 - fid, color::ACCENT_YELLOW, |v| format!("{:.8}", v));
                ui.add_space(space::MD);
                hairline(ui);
                ui.add_space(space::MD);
            }

            // ── Basis state probabilities ──
            section_header(ui, "basis state probabilities");
            ui.add_space(space::SM);

            let probs = simulation::basis_probabilities(&sim.statevector);
            let max_prob = probs.iter().cloned().fold(0.0f32, f32::max).max(0.01);

            let bits = n;
            let dim = probs.len();
            let max_rows = (dim as u32).min(128);

            for i in 0..max_rows as usize {
                let p = probs[i];
                let frac = p / max_prob;
                let bar_color = if p > 0.01 {
                    color::ACCENT_PURPLE
                } else if p > 0.001 {
                    color::ACCENT_PURPLE.linear_multiply(0.5)
                } else {
                    color::TEXT_DIM
                };

                ui.horizontal(|ui| {
                    let label = format!("|{:0>width$}⟩", format!("{:b}", i), width = bits);
                    ui.label(RichText::new(label).monospace().color(color::TEXT_PRIMARY).size(11.0));
                    ui.add_space(space::SM);

                    let bar_w = 120.0 * frac + 1.0;
                    let bar_h = 10.0;
                    let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(bar_w.max(2.0), bar_h), egui::Sense::hover());
                    ui.painter().rect_filled(bar_rect, CornerRadius::same(2), bar_color);

                    ui.add_space(space::SM);
                    ui.label(
                        RichText::new(format!("{:.4}", p))
                            .monospace()
                            .color(if p > 0.001 { color::TEXT_PRIMARY } else { color::TEXT_DIM })
                            .size(11.0),
                    );

                    // Show compare overlay if available
                    if let Some(ref cmp) = state.compare_simulation {
                        let cmp_probs = simulation::basis_probabilities(&cmp.statevector);
                        if i < cmp_probs.len() {
                            let cp = cmp_probs[i];
                            ui.add_space(space::SM);
                            let diff = p - cp;
                            let diff_color = if diff.abs() < 0.001 {
                                color::TEXT_DIM
                            } else if diff > 0.0 {
                                color::ACCENT_RED
                            } else {
                                color::ACCENT_GREEN
                            };
                            ui.label(
                                RichText::new(format!("{:+.4}", diff))
                                    .monospace()
                                    .color(diff_color)
                                    .size(11.0),
                            );
                        }
                    }
                });
            }

            if dim > max_rows as usize {
                ui.add_space(space::SM);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(format!("… {} more basis states", dim - max_rows as usize))
                            .color(color::TEXT_DIM)
                            .monospace()
                            .size(11.0),
                    );
                });
            }
        });
}

fn metric_card(ui: &mut egui::Ui, label: &str, value: f32, accent: egui::Color32, fmt: fn(f32) -> String) {
    egui::Frame::NONE
        .fill(color::BG_ELEVATED)
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(space::MD as i8, space::SM as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(label).color(color::TEXT_MUTED).monospace().size(12.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(fmt(value)).color(accent).monospace().size(14.0));
                });
            });
        });
}

fn section_header(ui: &mut egui::Ui, label: &str) {
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(label).color(color::TEXT_MUTED).monospace().size(11.0));
    });
}

fn hairline(ui: &mut egui::Ui) {
    let available = ui.available_width();
    let margin = (available * 0.1).min(40.0);
    let line_w = available - 2.0 * margin;
    ui.vertical_centered(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(line_w, 1.0), egui::Sense::hover());
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(0),
            Stroke::new(1.0, color::GRID_LINE),
            StrokeKind::Inside,
        );
    });
}
