// Entanglement metrics panel.
//
// Shows per-qubit von Neumann entropy (entanglement entropy) and a visual
// summary of quantum correlations. High entropy on a qubit indicates it is
// strongly entangled with the rest of the system.

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

    let entropies = simulation::qubit_entropies(&sim.statevector, n);
    let max_entropy = entropies.iter().cloned().fold(0.0f32, f32::max).max(0.01);
    let total_entropy: f32 = entropies.iter().sum();

    ui.vertical_centered(|ui| {
        ui.add_space(space::SM);
        ui.label(RichText::new("entanglement metrics").color(color::TEXT_MUTED).monospace().size(11.0));
        ui.add_space(space::SM);
    });

    hairline(ui);
    ui.add_space(space::MD);

    egui::ScrollArea::vertical()
        .id_salt("entanglement_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // ── Summary card ──
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(format!("total entropy  {:.4}", total_entropy))
                        .color(color::ACCENT_YELLOW)
                        .monospace()
                        .size(14.0),
                );
                let avg = if n > 0 { total_entropy / n as f32 } else { 0.0 };
                ui.label(
                    RichText::new(format!("avg per qubit  {:.4}", avg))
                        .color(color::TEXT_MUTED)
                        .monospace()
                        .size(12.0),
                );
            });
            ui.add_space(space::MD);
            hairline(ui);
            ui.add_space(space::MD);

            // ── Per-qubit entropy bars ──
            section_header(ui, "per-qubit entropy");
            ui.add_space(space::SM);

            let bar_max_w = 180.0;
            for i in 0..n {
                let e = entropies[i];
                let frac = if max_entropy > 0.0 { e / max_entropy.max(0.01) } else { 0.0 };

                let bar_color = if e > 0.8 {
                    color::ACCENT_RED
                } else if e > 0.5 {
                    color::ACCENT_YELLOW
                } else if e > 0.1 {
                    color::ACCENT_PURPLE
                } else {
                    color::ACCENT_GREEN
                };

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("q[{}]", i))
                            .monospace()
                            .color(color::TEXT_PRIMARY)
                            .size(12.0),
                    );
                    ui.add_space(space::SM);

                    let bar_w = (bar_max_w * frac).max(2.0);
                    let bar_h = 14.0;
                    let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(bar_w, bar_h), egui::Sense::hover());
                    let bg_rect = egui::Rect::from_min_size(bar_rect.min, Vec2::new(bar_max_w, bar_h));
                    ui.painter().rect_filled(bg_rect, CornerRadius::same(3), color::BG);
                    if bar_w > 0.0 {
                        let fill_rect = egui::Rect::from_min_size(bar_rect.min, Vec2::new(bar_w, bar_h));
                        ui.painter().rect_filled(fill_rect, CornerRadius::same(3), bar_color);
                    }

                    ui.add_space(space::SM);
                    ui.label(
                        RichText::new(format!("{:.4}", e))
                            .monospace()
                            .color(bar_color)
                            .size(12.0),
                    );

                    // Compare overlay
                    if let Some(ref cmp) = state.compare_simulation {
                        let cmp_ent = simulation::qubit_entropies(&cmp.statevector, n);
                        if i < cmp_ent.len() {
                            ui.add_space(space::SM);
                            let diff = e - cmp_ent[i];
                            let diff_color = if diff.abs() < 0.01 {
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
                ui.add_space(2.0);
            }
            ui.add_space(space::MD);
            hairline(ui);
            ui.add_space(space::MD);

            // ── Entanglement interpretation ──
            section_header(ui, "guide");
            ui.add_space(space::SM);
            ui.label(
                RichText::new("S = 0     pure state (no entanglement)")
                    .monospace()
                    .color(color::ACCENT_GREEN)
                    .size(11.0),
            );
            ui.label(
                RichText::new("S = 1     maximally entangled (Bell pair)")
                    .monospace()
                    .color(color::ACCENT_RED)
                    .size(11.0),
            );
            ui.label(
                RichText::new("S(ρᵢ)     von Neumann entropy of reduced state")
                    .monospace()
                    .color(color::TEXT_DIM)
                    .size(11.0),
            );
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
