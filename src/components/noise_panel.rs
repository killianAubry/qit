// Noise control panel.
//
// Controls for noise models, per-gate injection, and device calibration.
// Matches the app's dense, monospace, dev-tool aesthetic — text-driven
// layout with compact controls and subtle visual indicators.

use crate::state::noise::{CalibrationSource, NoiseConfig};
use crate::state::AppState;
use crate::theme::{color, space};
use egui::{CornerRadius, RichText, Stroke, StrokeKind};

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    let cfg = &mut state.noise_config;

    // ── header ──
    ui.horizontal(|ui| {
        ui.label(RichText::new("noise").color(color::TEXT_MUTED).monospace());
        ui.label(RichText::new("·").color(color::TEXT_DIM));
        let status = if cfg.noise_enabled {
            RichText::new("active").color(color::ACCENT_GREEN)
        } else {
            RichText::new("disabled").color(color::TEXT_DIM)
        };
        ui.label(status.monospace().size(12.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(format!(
                    "{} models",
                    [cfg.depolarizing_enabled, cfg.amplitude_damping_enabled, cfg.phase_damping_enabled]
                        .iter()
                        .filter(|&&e| e)
                        .count()
                ))
                .color(color::TEXT_DIM)
                .monospace()
                .size(11.0),
            );
        });
    });
    ui.add_space(space::SM);

    egui::ScrollArea::vertical()
        .id_salt("noise_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // ── global enable ──
            global_toggle(ui, cfg);
            ui.add_space(space::MD);
            hairline(ui);
            ui.add_space(space::MD);

            // ── noise models ──
            section_label(ui, "models");
            models_grid(ui, cfg);
            ui.add_space(space::MD);
            hairline(ui);
            ui.add_space(space::MD);

            // ── device params ──
            section_label(ui, "device");
            device_grid(ui, cfg);
            ui.add_space(space::MD);
            hairline(ui);
            ui.add_space(space::MD);

            // ── per-gate noise ──
            section_label(ui, "per-gate");
            per_gate_table(ui, cfg);
            ui.add_space(space::MD);
            hairline(ui);
            ui.add_space(space::MD);

            // ── calibration import ──
            section_label(ui, "calibration");
            calibration_section(ui, cfg);
        });
}

// ── global toggle ──────────────────────────────────────────────────────

fn global_toggle(ui: &mut egui::Ui, cfg: &mut NoiseConfig) {
    ui.horizontal(|ui| {
        let resp = ui.selectable_label(cfg.noise_enabled, "");
        if resp.clicked() {
            cfg.noise_enabled = !cfg.noise_enabled;
        }
        let label_color = if cfg.noise_enabled {
            color::ACCENT_YELLOW
        } else {
            color::TEXT_DIM
        };
        let label = if cfg.noise_enabled {
            "apply noise to simulation"
        } else {
            "noise bypassed (run ⌘R to see effect)"
        };
        ui.label(
            RichText::new(label)
                .color(label_color)
                .monospace()
                .size(12.0),
        );
    });
}

// ── noise models ───────────────────────────────────────────────────────

fn models_grid(ui: &mut egui::Ui, cfg: &mut NoiseConfig) {
    egui::Grid::new("noise_models")
        .num_columns(3)
        .spacing([space::MD, 8.0])
        .show(ui, |ui| {
            model_row(ui, "depol", "depolarizing", &mut cfg.depolarizing_enabled, &mut cfg.depolarizing_probability);
            model_row(ui, "T1", "amplitude damp", &mut cfg.amplitude_damping_enabled, &mut cfg.amplitude_damping_gamma);
            model_row(ui, "T2", "phase damp", &mut cfg.phase_damping_enabled, &mut cfg.phase_damping_gamma);
        });
}

fn model_row(
    ui: &mut egui::Ui,
    abbr: &str,
    label: &str,
    enabled: &mut bool,
    value: &mut f32,
) {
    let resp = ui.selectable_label(*enabled, "");
    if resp.clicked() {
        *enabled = !*enabled;
    }
    let col = if *enabled {
        color::ACCENT_YELLOW
    } else {
        color::TEXT_DIM
    };
    ui.label(RichText::new(abbr).color(col).monospace().size(12.0));
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(color::TEXT_MUTED).monospace());
        ui.add_space(space::XS);
        ui.label(
            RichText::new(format!("{:.4}", *value))
                .color(color::TEXT_PRIMARY)
                .monospace()
                .size(11.0),
        );
        ui.add(
            egui::DragValue::new(value)
                .speed(0.0005)
                .range(0.0..=0.5)
                .suffix("")
                ,
        );
    });
    ui.end_row();
}

// ── device params ──────────────────────────────────────────────────────

fn device_grid(ui: &mut egui::Ui, cfg: &mut NoiseConfig) {
    egui::Grid::new("device_params")
        .num_columns(4)
        .spacing([space::MD, 8.0])
        .show(ui, |ui| {
            device_row(ui, "T1", &mut cfg.device_t1_us, 1.0..=500.0, "µs", "relaxation", 50.0);
            device_row(ui, "T2", &mut cfg.device_t2_us, 1.0..=500.0, "µs", "dephasing", 70.0);
            device_row(ui, "gate", &mut cfg.device_gate_error_rate, 0.0..=0.1, "", "error rate", 0.001);
            device_row(ui, "ro", &mut cfg.device_readout_error_rate, 0.0..=0.5, "", "readout err", 0.01);
            device_row(ui, "freq", &mut cfg.device_qubit_frequency_ghz, 3.0..=8.0, "GHz", "frequency", 5.0);
        });
}

fn device_row(
    ui: &mut egui::Ui,
    abbr: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
    desc: &str,
    default: f32,
) {
    let max = *range.end();
    let min = *range.start();
    ui.label(RichText::new(abbr).color(color::ACCENT_YELLOW).monospace());
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{:.2}{}", *value, suffix))
                .color(color::TEXT_PRIMARY)
                .monospace(),
        );
        ui.add(
            egui::DragValue::new(value)
                .speed(default * 0.1)
                .range(range)
                .suffix(""),
        );
    });
    let frac = ((*value - min) / (max - min).max(1e-6)).clamp(0.0, 1.0);
    let fill = if frac > 0.8 {
        color::ACCENT_RED.linear_multiply(0.6)
    } else if frac > 0.5 {
        color::ACCENT_YELLOW.linear_multiply(0.5)
    } else {
        color::ACCENT_PURPLE.linear_multiply(0.4)
    };
    ui.add(egui::ProgressBar::new(frac as f32).desired_width(48.0).fill(fill).show_percentage());
    ui.label(
        RichText::new(desc)
            .color(color::TEXT_DIM)
            .monospace()
            .size(11.0),
    );
    ui.end_row();
}

// ── per-gate noise ─────────────────────────────────────────────────────

fn per_gate_table(ui: &mut egui::Ui, cfg: &mut NoiseConfig) {
    cfg.ensure_default_gates();

    // Column headers
    egui::Grid::new("per_gate_hdr")
        .num_columns(3)
        .spacing([space::MD, 6.0])
        .show(ui, |ui| {
            ui.label(
                RichText::new("gate")
                    .color(color::TEXT_DIM)
                    .monospace()
                    .size(11.0),
            );
            ui.label(
                RichText::new("depol")
                    .color(color::TEXT_DIM)
                    .monospace()
                    .size(11.0),
            );
            ui.label(
                RichText::new("damp")
                    .color(color::TEXT_DIM)
                    .monospace()
                    .size(11.0),
            );
            ui.end_row();
        });

    let gate_keys: Vec<String> = cfg.noise_per_gate.keys().cloned().collect();

    egui::Grid::new("per_gate_data")
        .num_columns(3)
        .spacing([space::MD, 4.0])
        .striped(true)
        .show(ui, |ui| {
            for name in gate_keys {
                if let Some(params) = cfg.noise_per_gate.get_mut(&name) {
                    ui.label(
                        RichText::new(&name)
                            .color(color::TEXT_PRIMARY)
                            .monospace()
                            .size(12.0),
                    );
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut params.depolarizing_prob)
                                .speed(0.0001)
                                .range(0.0..=0.5)
                                .suffix("")
                                ,
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut params.damping_gamma)
                                .speed(0.0001)
                                .range(0.0..=0.5)
                                .suffix("")
                                ,
                        );
                    });
                    ui.end_row();
                }
            }
        });
}

// ── calibration import ─────────────────────────────────────────────────

fn calibration_section(ui: &mut egui::Ui, cfg: &mut NoiseConfig) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("src").color(color::TEXT_MUTED).monospace());
        egui::ComboBox::new("cal_src", "")
            .selected_text(RichText::new(cfg.calibration_source.label()).color(color::TEXT_PRIMARY).monospace().text().to_string())
            .show_ui(ui, |ui| {
                for src in &[
                    CalibrationSource::None,
                    CalibrationSource::Qiskit,
                    CalibrationSource::Cirq,
                    CalibrationSource::Custom,
                ] {
                    ui.selectable_value(
                        &mut cfg.calibration_source,
                        *src,
                        RichText::new(src.label()).monospace().text().to_string(),
                    );
                }
            });
    });

    if cfg.calibration_source != CalibrationSource::None {
        ui.add_space(space::SM);
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new("import").monospace().size(12.0))
                .clicked()
            {
                cfg.calibration_imported = true;
                cfg.calibration_device_name = match cfg.calibration_source {
                    CalibrationSource::Qiskit => "IBM_Eagle_r3".to_string(),
                    CalibrationSource::Cirq => "Sycamore".to_string(),
                    CalibrationSource::Custom => "custom_device".to_string(),
                    _ => String::new(),
                };
            }
            if cfg.calibration_imported {
                ui.label(RichText::new("·").color(color::TEXT_DIM));
                ui.label(
                    RichText::new(&cfg.calibration_device_name)
                        .color(color::ACCENT_GREEN)
                        .monospace()
                        .size(12.0),
                );
            }
        });
    }
}

// ── helpers ────────────────────────────────────────────────────────────

fn section_label(ui: &mut egui::Ui, label: &str) {
    ui.label(
        RichText::new(label)
            .color(color::TEXT_MUTED)
            .monospace()
            .size(12.0),
    );
    ui.add_space(space::XS);
}

fn hairline(ui: &mut egui::Ui) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(0),
        Stroke::new(1.0, color::GRID_LINE),
        StrokeKind::Inside,
    );
}
