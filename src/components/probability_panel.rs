// Probability distribution panel.
//
// One row per basis state with a horizontal bar showing |amplitude|².
// Uses direct painter calls (instead of one widget per row) so it stays
// performant up to 2^10 = 1024 rows.
//
// In compare mode, each row shows two overlaid bars — primary simulator
// on top (thinner, brighter) and compare simulator behind (full height,
// green-tinted).

use egui::{CornerRadius, FontId, Pos2, Rect, Sense, Vec2};

use crate::state::AppState;
use crate::theme::{color, space};

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    let has_compare = state.compare_simulation.is_some();

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("probabilities")
                .color(color::TEXT_MUTED)
                .monospace(),
        );
        if has_compare {
            ui.add_space(space::SM);
            ui.label(
                egui::RichText::new("—")
                    .color(color::TEXT_DIM)
                    .monospace(),
            );
            ui.add_space(space::SM);
            ui.label(
                egui::RichText::new(state.simulator.label())
                    .color(color::ACCENT_YELLOW)
                    .monospace()
                    .size(11.0),
            );
            ui.add_space(space::XS);
            ui.label(
                egui::RichText::new("vs")
                    .color(color::TEXT_DIM)
                    .monospace()
                    .size(11.0),
            );
            ui.add_space(space::XS);
            if let Some(cmp) = state.compare_simulator {
                ui.label(
                    egui::RichText::new(cmp.label())
                        .color(color::ACCENT_GREEN)
                        .monospace()
                        .size(11.0),
                );
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!("{} states", state.simulation.probabilities.len()))
                    .color(color::TEXT_DIM)
                    .monospace(),
            );
        });
    });
    ui.add_space(space::SM);

    let probs = &state.simulation.probabilities;
    let n = probs.len();
    let qubits = state.simulation.num_qubits.max(1);

    let row_h = 16.0;
    let label_w = (qubits as f32 * 8.5 + 18.0).max(56.0);
    let val_w = if has_compare { 108.0 } else { 52.0 };

    egui::ScrollArea::vertical()
        .id_salt("probability_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let avail_w = ui.available_width().max(180.0);
            let (rect, _) = ui.allocate_exact_size(
                Vec2::new(avail_w, n as f32 * row_h + 4.0),
                Sense::hover(),
            );
            let painter = ui.painter_at(rect);

            let bar_x = rect.min.x + label_w;
            let bar_w_max = (rect.max.x - val_w - bar_x - 4.0).max(20.0);
            let max_p = probs.iter().copied().fold(0.0_f32, f32::max).max(1e-6);

            let cmp_probs: Option<&Vec<f32>> = state.compare_simulation.as_ref().map(|s| &s.probabilities);

            for (i, &p) in probs.iter().enumerate() {
                let y = rect.min.y + i as f32 * row_h + row_h * 0.5;

                painter.text(
                    Pos2::new(rect.min.x + 4.0, y),
                    egui::Align2::LEFT_CENTER,
                    format!("|{:0>width$b}⟩", i, width = qubits),
                    FontId::monospace(11.0),
                    color::TEXT_MUTED,
                );

                // Background track
                let bar_full = Rect::from_min_size(
                    Pos2::new(bar_x, y - 5.0),
                    Vec2::new(bar_w_max, 10.0),
                );
                painter.rect_filled(bar_full, CornerRadius::same(2), color::BG_ELEVATED);

                // Compare bar (behind, full height, green)
                if let Some(cp) = cmp_probs {
                    if let Some(&cp_val) = cp.get(i) {
                        let cmp_max = cp.iter().copied().fold(0.0_f32, f32::max).max(1e-6);
                        let cmp_fill = bar_w_max * (cp_val / cmp_max);
                        let cmp_rect = Rect::from_min_size(
                            Pos2::new(bar_x, y - 5.0),
                            Vec2::new(cmp_fill, 10.0),
                        );
                        painter.rect_filled(cmp_rect, CornerRadius::same(2), color::ACCENT_GREEN.linear_multiply(0.55));
                    }
                }

                // Primary bar (on top, narrower, brighter)
                let fill_w = bar_w_max * (p / max_p);
                let inner_h = if has_compare { 5.0 } else { 10.0 };
                let inner_y = if has_compare { y - 2.5 } else { y - 5.0 };
                let bar_filled = Rect::from_min_size(
                    Pos2::new(bar_x, inner_y),
                    Vec2::new(fill_w, inner_h),
                );
                let bar_color = if p == max_p {
                    color::ACCENT_YELLOW
                } else {
                    color::ACCENT_PURPLE
                };
                painter.rect_filled(bar_filled, CornerRadius::same(2), bar_color);

                // Values
                if has_compare {
                    if let Some(cp) = cmp_probs {
                        if let Some(&cp_val) = cp.get(i) {
                            painter.text(
                                Pos2::new(rect.max.x - 56.0, y),
                                egui::Align2::RIGHT_CENTER,
                                format!("{:.3}", cp_val),
                                FontId::monospace(10.0),
                                color::ACCENT_GREEN.linear_multiply(0.7),
                            );
                        }
                    }
                    painter.text(
                        Pos2::new(rect.max.x - 4.0, y),
                        egui::Align2::RIGHT_CENTER,
                        format!("{:.3}", p),
                        FontId::monospace(10.0),
                        color::ACCENT_YELLOW,
                    );
                } else {
                    painter.text(
                        Pos2::new(rect.max.x - 4.0, y),
                        egui::Align2::RIGHT_CENTER,
                        format!("{:.3}", p),
                        FontId::monospace(11.0),
                        color::TEXT_PRIMARY,
                    );
                }
            }
        });
}
