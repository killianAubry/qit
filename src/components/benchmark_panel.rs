use crate::state::AppState;
use crate::theme::{color, space};

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("benchmarks")
                .color(color::TEXT_MUTED)
                .monospace(),
        );
    });
    ui.add_space(space::SM);

    let runtime = state.simulation.run_time_ms.unwrap_or(0.0);
    let overhead = state.simulation.memory_mb.unwrap_or(0.0);

    ui.label(format!("Runtime: {:.2} ms", runtime));
    ui.label(format!("Memory: {:.2} MB", overhead));
}
