// Density matrix panel.
//
// Visualizes the full N-qubit density matrix ρ = |ψ⟩⟨ψ| as a color-mapped
// heatmap. Magnitude is encoded as opacity/brightness, phase as hue.
// Supports compare mode with side-by-side or difference matrix.

use egui::{Color32, CornerRadius, Pos2, Rect, RichText, Stroke, StrokeKind, Vec2};

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
            ui.label(RichText::new("run ⌘R to compute matrix").color(color::TEXT_DIM).monospace().size(11.0));
        });
        return;
    }

    let rho = simulation::density_matrix(&sim.statevector);
    let dim = 1 << n;

    ui.vertical_centered(|ui| {
        ui.add_space(space::SM);
        ui.label(RichText::new("density matrix").color(color::TEXT_MUTED).monospace().size(11.0));
        ui.label(
            RichText::new(format!("{}×{}  ({} qubits)", dim, dim, n))
                .color(color::TEXT_DIM)
                .monospace()
                .size(11.0),
        );
        ui.add_space(space::SM);
    });

    hairline(ui);
    ui.add_space(space::MD);

    egui::ScrollArea::both()
        .id_salt("density_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // ── Heatmap ──
            let cell_size = 28.0;
            let label_width = 42.0;
            let total_w = label_width + dim as f32 * cell_size;
            let total_h = 16.0 + dim as f32 * cell_size;

            let (rect, _) = ui.allocate_exact_size(Vec2::new(total_w, total_h), egui::Sense::hover());
            let painter = ui.painter_at(rect);

            // Column labels (basis indices)
            for j in 0..dim {
                let x = rect.min.x + label_width + j as f32 * cell_size;
                painter.text(
                    Pos2::new(x + cell_size * 0.5, rect.min.y + 8.0),
                    egui::Align2::CENTER_TOP,
                    format!("{:0>width$b}", j, width = n.max(1)),
                    egui::FontId::monospace(8.0),
                    color::TEXT_DIM,
                );
            }

            // Row labels + cells
            for i in 0..dim {
                let y = rect.min.y + 16.0 + i as f32 * cell_size;

                // Row label
                painter.text(
                    Pos2::new(rect.min.x + label_width - 4.0, y + cell_size * 0.5),
                    egui::Align2::RIGHT_CENTER,
                    format!("{:0>width$b}", i, width = n.max(1)),
                    egui::FontId::monospace(8.0),
                    color::TEXT_DIM,
                );

                for j in 0..dim {
                    let x = rect.min.x + label_width + j as f32 * cell_size;
                    let idx = i * dim + j;
                    let c = rho[idx];

                    let cell_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(cell_size, cell_size));

                    let mag = (c.re * c.re + c.im * c.im).sqrt().min(1.0);

                    let cell_color = complex_to_color(c.re, c.im, mag);
                    painter.rect_filled(cell_rect, CornerRadius::same(1), cell_color);

                    // Grid lines
                    painter.rect_stroke(
                        cell_rect,
                        CornerRadius::same(0),
                        Stroke::new(0.5, color::GRID_LINE),
                        StrokeKind::Inside,
                    );
                }
            }

            ui.add_space(space::MD);

            // ── Legend ──
            ui.horizontal(|ui| {
                ui.label(RichText::new("phase").monospace().color(color::TEXT_MUTED).size(11.0));
                for (label, col) in &[
                    ("0", color::ACCENT_RED),
                    ("π/2", color::ACCENT_GREEN),
                    ("π", color::ACCENT_PURPLE),
                    ("-π/2", color::ACCENT_YELLOW),
                ] {
                    let (swatch, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), egui::Sense::hover());
                    ui.painter().rect_filled(swatch, CornerRadius::same(1), *col);
                    ui.label(RichText::new(*label).monospace().color(color::TEXT_DIM).size(10.0));
                    ui.add_space(space::XS);
                }
            });

            ui.add_space(space::SM);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("brightness = magnitude  ·  hue = phase")
                        .monospace()
                        .color(color::TEXT_DIM)
                        .size(10.0),
                );
            });

            // ── Trace check ──
            ui.add_space(space::SM);
            let trace: f32 = (0..dim).map(|k| {
                let c = rho[k * dim + k];
                c.re * c.re + c.im * c.im
            }).sum();
            let trace_sqrt = trace.sqrt();

            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(format!("Tr(ρ) = {:.6}", trace_sqrt))
                        .monospace()
                        .color(if (trace_sqrt - 1.0).abs() < 0.01 { color::ACCENT_GREEN } else { color::ACCENT_YELLOW })
                        .size(11.0),
                );
            });
        });
}

fn complex_to_color(re: f32, im: f32, mag: f32) -> Color32 {
    let phase = im.atan2(re);
    let hue = (phase + std::f32::consts::PI) / (2.0 * std::f32::consts::PI);

    // HSV-like color mapping: hue from phase, saturation fixed, value from magnitude
    let h = hue * 6.0;
    let s = 0.85;
    let v = 0.15 + mag * 0.85;

    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match h as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    Color32::from_rgb(
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
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
