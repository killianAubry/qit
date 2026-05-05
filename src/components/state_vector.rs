// State vector display.
//
// One row per basis state: `|bin⟩  +re  +imi`. Read-only; the data comes
// straight from `state.simulation`.

use crate::state::AppState;
use crate::theme::{color, space};

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("state vector")
                .color(color::TEXT_MUTED)
                .monospace(),
        );
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
                });
            }
        });
}
