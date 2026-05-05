use egui::RichText;
use crate::state::AppState;
use crate::theme::{color, space};

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("performance metrics")
                .color(color::TEXT_MUTED)
                .monospace(),
        );
    });
    ui.add_space(space::MD);

    let runtime = state.simulation.run_time_ms.unwrap_or(0.0);
    let memory_mb = state.simulation.memory_mb.unwrap_or(0.0);
    let num_qubits = state.simulation.num_qubits;
    let num_states = 1usize << num_qubits.min(20); // Prevent overflow on display
    
    egui::ScrollArea::vertical()
        .id_salt("benchmark_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            
            // 1. Simulation Metrics
            ui.label(RichText::new("Simulation").color(color::TEXT_PRIMARY).strong().monospace());
            ui.add_space(space::SM);
            
            egui::Grid::new("bench_sim_grid")
                .num_columns(2)
                .spacing([40.0, 12.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Runtime").color(color::TEXT_MUTED).monospace());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{:.2} ms", runtime)).color(color::ACCENT_YELLOW).monospace());
                        
                        let progress = (runtime / 100.0).clamp(0.0, 1.0) as f32; // Normalize visually
                        ui.add(egui::ProgressBar::new(progress)
                            .show_percentage()
                            .desired_width(100.0)
                            .fill(if progress > 0.8 { color::ACCENT_RED } else { color::ACCENT_YELLOW })
                        );
                    });
                    ui.end_row();

                    ui.label(RichText::new("Qubits").color(color::TEXT_MUTED).monospace());
                    ui.label(RichText::new(format!("{}", num_qubits)).color(color::TEXT_PRIMARY).monospace());
                    ui.end_row();

                    ui.label(RichText::new("State Size").color(color::TEXT_MUTED).monospace());
                    let size_color = if num_qubits >= 10 { color::ACCENT_RED } else { color::TEXT_PRIMARY };
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("2^{} ({} states)", num_qubits, num_states)).color(size_color).monospace());
                        if num_qubits >= 10 {
                            ui.label(RichText::new("⚠ scaling limit").color(color::ACCENT_RED).monospace());
                        }
                    });
                    ui.end_row();
                });

            ui.add_space(space::LG);

            // 2. System Metrics
            ui.label(RichText::new("System").color(color::TEXT_PRIMARY).strong().monospace());
            ui.add_space(space::SM);

            egui::Grid::new("bench_sys_grid")
                .num_columns(2)
                .spacing([40.0, 12.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Memory").color(color::TEXT_MUTED).monospace());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{:.4} MB", memory_mb)).color(color::ACCENT_PURPLE).monospace());
                        
                        let mem_progress = (memory_mb / 10.0).clamp(0.0, 1.0) as f32;
                        ui.add(egui::ProgressBar::new(mem_progress)
                            .desired_width(100.0)
                            .fill(color::ACCENT_PURPLE)
                        );
                    });
                    ui.end_row();
                    
                    ui.label(RichText::new("Engine Mode").color(color::TEXT_MUTED).monospace());
                    ui.label(RichText::new(state.simulator.label()).color(color::TEXT_PRIMARY).monospace());
                    ui.end_row();
                });
                
            ui.add_space(space::LG);

            // 3. Circuit Stats
            ui.label(RichText::new("Circuit").color(color::TEXT_PRIMARY).strong().monospace());
            ui.add_space(space::SM);
            
            egui::Grid::new("bench_circ_grid")
                .num_columns(2)
                .spacing([40.0, 12.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Total Gates").color(color::TEXT_MUTED).monospace());
                    ui.label(RichText::new(format!("{}", state.circuit.gates.len())).color(color::TEXT_PRIMARY).monospace());
                    ui.end_row();

                    ui.label(RichText::new("Depth (Steps)").color(color::TEXT_MUTED).monospace());
                    ui.label(RichText::new(format!("{}", state.circuit.num_steps)).color(color::TEXT_PRIMARY).monospace());
                    ui.end_row();
                });
        });
}
