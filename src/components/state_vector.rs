// State vector display.
//
// One row per basis state: `|bin⟩  +re  +imi`. Read-only; the data comes
// straight from `state.simulation`.
//
// In compare mode, each row shows two columns of amplitudes — primary
// simulator values on the left (yellow/purple) and compare simulator values
// on the right (green).

use crate::state::AppState;
use crate::theme::{color, space};

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    let has_compare = state.compare_simulation.is_some();

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("state vector")
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
                egui::RichText::new(format!("{} amps", state.simulation.statevector.len()))
                    .color(color::TEXT_DIM)
                    .monospace(),
            );
        });
    });
    ui.add_space(space::SM);

    let qubits = state.simulation.num_qubits.max(1);
    let amps = &state.simulation.statevector;
    let cmp_amps: Option<&Vec<crate::state::simulation::Complex>> =
        state.compare_simulation.as_ref().map(|s| &s.statevector);

    egui::ScrollArea::vertical()
        .id_salt("statevector_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            for (i, c) in amps.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("|{:0>width$b}⟩", i, width = qubits))
                            .monospace()
                            .color(color::TEXT_MUTED),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:>+.3}", c.re))
                            .monospace()
                            .color(color::TEXT_PRIMARY),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:>+.3}i", c.im))
                            .monospace()
                            .color(color::ACCENT_PURPLE),
                    );

                    if let Some(ca) = cmp_amps {
                        ui.add_space(space::MD);
                        ui.label(
                            egui::RichText::new("│")
                                .monospace()
                                .color(color::TEXT_DIM),
                        );
                        ui.add_space(space::MD);
                        if let Some(cc) = ca.get(i) {
                            ui.label(
                                egui::RichText::new(format!("{:>+.3}", cc.re))
                                    .monospace()
                                    .color(color::ACCENT_GREEN),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:>+.3}i", cc.im))
                                    .monospace()
                                    .color(color::ACCENT_GREEN.linear_multiply(0.7)),
                            );
                        }
                    }
                });
            }
        });
}
