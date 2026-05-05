// Probability distribution panel.
//
// One row per basis state with a horizontal bar showing |amplitude|².
// Uses direct painter calls (instead of one widget per row) so it stays
// performant up to 2^10 = 1024 rows.

use egui::{CornerRadius, FontId, Pos2, Rect, Sense, Vec2};

use crate::state::AppState;
use crate::theme::{color, space};

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("probabilities")
                .color(color::TEXT_MUTED)
                .monospace(),
        );
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
    let val_w = 52.0;

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

            for (i, &p) in probs.iter().enumerate() {
                let y = rect.min.y + i as f32 * row_h + row_h * 0.5;

                painter.text(
                    Pos2::new(rect.min.x + 4.0, y),
                    egui::Align2::LEFT_CENTER,
                    format!("|{:0>width$b}⟩", i, width = qubits),
                    FontId::monospace(11.0),
                    color::TEXT_MUTED,
                );

                let bar_full = Rect::from_min_size(
                    Pos2::new(bar_x, y - 5.0),
                    Vec2::new(bar_w_max, 10.0),
                );
                painter.rect_filled(bar_full, CornerRadius::same(2), color::BG_ELEVATED);

                let fill_w = bar_w_max * (p / max_p);
                let bar_filled = Rect::from_min_size(
                    Pos2::new(bar_x, y - 5.0),
                    Vec2::new(fill_w, 10.0),
                );
                let bar_color = if p == max_p {
                    color::ACCENT_YELLOW
                } else {
                    color::ACCENT_PURPLE
                };
                painter.rect_filled(bar_filled, CornerRadius::same(2), bar_color);

                painter.text(
                    Pos2::new(rect.max.x - 4.0, y),
                    egui::Align2::RIGHT_CENTER,
                    format!("{:.3}", p),
                    FontId::monospace(11.0),
                    color::TEXT_PRIMARY,
                );
            }
        });
}
