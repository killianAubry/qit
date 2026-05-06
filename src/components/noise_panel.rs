// Noise control panel.
//
// Controls for noise models, per-gate injection, and device calibration.
// Redesigned with visual indicators, centered layouts, and progress bars.

use crate::state::noise::{CalibrationSource, NoiseConfig};
use crate::state::AppState;
use crate::theme::{color, space};
use egui::{CornerRadius, RichText, Stroke, StrokeKind, Vec2};

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    let cfg = &mut state.noise_config;

    // ── header ──
    ui.vertical_centered(|ui| {
        ui.add_space(space::SM);
        let (status_text, status_color) = if cfg.noise_enabled {
            ("●  noise active", color::ACCENT_GREEN)
        } else {
            ("○  noise bypassed", color::TEXT_DIM)
        };
        ui.label(RichText::new(status_text).color(status_color).monospace().size(13.0));

        let active_count = [cfg.depolarizing_enabled, cfg.amplitude_damping_enabled, cfg.phase_damping_enabled]
            .iter()
            .filter(|&&e| e)
            .count();
        ui.label(
            RichText::new(format!("{} model{} active", active_count, if active_count == 1 { "" } else { "s" }))
                .color(color::TEXT_DIM)
                .monospace()
                .size(11.0),
        );
        ui.add_space(space::SM);
    });

    hairline(ui);
    ui.add_space(space::MD);

    egui::ScrollArea::vertical()
        .id_salt("noise_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // ── global enable ──
            ui.vertical_centered(|ui| {
                global_toggle(ui, cfg);
            });
            ui.add_space(space::MD);
            hairline(ui);
            ui.add_space(space::MD);

            // ── noise models ──
            section_header(ui, "noise models");
            ui.add_space(space::SM);
            models_panel(ui, cfg);
            ui.add_space(space::MD);
            hairline(ui);
            ui.add_space(space::MD);

            // ── device params ──
            section_header(ui, "device parameters");
            ui.add_space(space::SM);
            device_panel(ui, cfg);
            ui.add_space(space::MD);
            hairline(ui);
            ui.add_space(space::MD);

            // ── per-gate noise ──
            section_header(ui, "per-gate noise");
            ui.add_space(space::SM);
            per_gate_panel(ui, cfg);
            ui.add_space(space::MD);
            hairline(ui);
            ui.add_space(space::MD);

            // ── calibration ──
            section_header(ui, "calibration source");
            ui.add_space(space::SM);
            ui.vertical_centered(|ui| {
                calibration_section(ui, cfg);
            });
            ui.add_space(space::MD);
        });
}

// ── global toggle ──────────────────────────────────────────────────────

fn global_toggle(ui: &mut egui::Ui, cfg: &mut NoiseConfig) {
    let bg = if cfg.noise_enabled {
        color::ACCENT_GREEN.linear_multiply(0.15)
    } else {
        color::BG_ELEVATED
    };
    let border = if cfg.noise_enabled {
        color::ACCENT_GREEN.linear_multiply(0.4)
    } else {
        color::GRID_LINE
    };

    egui::Frame::NONE
        .fill(bg)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(space::LG as i8, space::MD as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let resp = ui.selectable_label(cfg.noise_enabled, "");
                if resp.clicked() {
                    cfg.noise_enabled = !cfg.noise_enabled;
                }
                ui.add_space(space::SM);
                let label = if cfg.noise_enabled {
                    RichText::new("noise enabled — run ⌘R to apply")
                        .color(color::ACCENT_GREEN)
                        .monospace()
                } else {
                    RichText::new("noise disabled — toggle to enable")
                        .color(color::TEXT_MUTED)
                        .monospace()
                };
                ui.label(label);
            });
        });
}

// ── noise models panel ─────────────────────────────────────────────────

fn models_panel(ui: &mut egui::Ui, cfg: &mut NoiseConfig) {
    egui::Frame::NONE
        .fill(color::BG_ELEVATED)
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::same(space::MD as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            model_card(ui, "depolarizing", "DEPOL",
                "Random Pauli X/Y/Z errors.\nModels gate and environmental noise\nas uniform error channel.",
                &mut cfg.depolarizing_enabled, &mut cfg.depolarizing_probability, 0.01);
            ui.add_space(space::SM);
            hairline(ui);
            ui.add_space(space::SM);
            model_card(ui, "amplitude damping", "T₁",
                "Energy relaxation |1⟩ → |0⟩.\nModels spontaneous emission and\nthermal relaxation processes.",
                &mut cfg.amplitude_damping_enabled, &mut cfg.amplitude_damping_gamma, 0.005);
            ui.add_space(space::SM);
            hairline(ui);
            ui.add_space(space::SM);
            model_card(ui, "phase damping", "T₂",
                "Pure dephasing without energy loss.\nModels loss of phase coherence\nin the qubit state.",
                &mut cfg.phase_damping_enabled, &mut cfg.phase_damping_gamma, 0.003);
        });
}

fn model_card(
    ui: &mut egui::Ui,
    _name: &str,
    abbr: &str,
    desc: &str,
    enabled: &mut bool,
    value: &mut f32,
    _default: f32,
) {
    ui.horizontal(|ui| {
        // Left: toggle + abbreviation
        let resp = ui.selectable_label(*enabled, "");
        if resp.clicked() {
            *enabled = !*enabled;
        }
        ui.add_space(space::SM);

        let col = if *enabled { color::ACCENT_YELLOW } else { color::TEXT_DIM };
        ui.label(RichText::new(abbr).color(col).monospace().size(13.0).strong());

        ui.add_space(space::MD);

        // Center: description
        ui.vertical(|ui| {
            for line in desc.lines() {
                ui.label(RichText::new(line).color(color::TEXT_MUTED).monospace().size(10.0));
            }
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Right: visual level indicator + value + slider
            let level_frac = (*value / 0.5).clamp(0.0, 1.0);
            let level_color = if level_frac < 0.2 {
                color::ACCENT_GREEN
            } else if level_frac < 0.5 {
                color::ACCENT_YELLOW
            } else {
                color::ACCENT_RED
            };

            // Mini visual bar
            let bar_w = 40.0;
            let bar_h = 8.0;
            let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(bar_w, bar_h), egui::Sense::hover());
            let fill_w = bar_w * level_frac;
            ui.painter().rect_filled(
                bar_rect,
                CornerRadius::same(2),
                color::BG,
            );
            if fill_w > 0.0 {
                let fill_rect = egui::Rect::from_min_size(bar_rect.min, Vec2::new(fill_w, bar_h));
                ui.painter().rect_filled(
                    fill_rect,
                    CornerRadius::same(2),
                    if *enabled { level_color } else { color::TEXT_DIM },
                );
            }

            ui.add_space(space::SM);
            ui.label(
                RichText::new(format!("{:.4}", *value))
                    .color(if *enabled { color::TEXT_PRIMARY } else { color::TEXT_DIM })
                    .monospace()
                    .size(12.0),
            );
            ui.add(
                egui::DragValue::new(value)
                    .speed(0.0005)
                    .range(0.0..=0.5)
                    .suffix(""),
            );
        });
    });
}

// ── device params panel ─────────────────────────────────────────────────

fn device_panel(ui: &mut egui::Ui, cfg: &mut NoiseConfig) {
    egui::Frame::NONE
        .fill(color::BG_ELEVATED)
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::same(space::MD as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            device_param_row(ui, "T₁", "relaxation time", &mut cfg.device_t1_us, 1.0..=500.0, "µs", 50.0);
            ui.add_space(space::SM);
            device_param_row(ui, "T₂", "dephasing time", &mut cfg.device_t2_us, 1.0..=500.0, "µs", 70.0);
            ui.add_space(space::SM);
            device_param_row(ui, "gate err", "gate error rate", &mut cfg.device_gate_error_rate, 0.0..=0.1, "", 0.001);
            ui.add_space(space::SM);
            device_param_row(ui, "ro err", "readout error", &mut cfg.device_readout_error_rate, 0.0..=0.5, "", 0.01);
            ui.add_space(space::SM);
            device_param_row(ui, "freq", "qubit frequency", &mut cfg.device_qubit_frequency_ghz, 3.0..=8.0, "GHz", 5.0);
        });
}

fn device_param_row(
    ui: &mut egui::Ui,
    abbr: &str,
    desc: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
    _default: f32,
) {
    let max = *range.end();
    let min = *range.start();
    let frac = ((*value - min) / (max - min).max(1e-6)).clamp(0.0, 1.0);

    ui.horizontal(|ui| {
        ui.label(RichText::new(abbr).color(color::ACCENT_PURPLE).monospace().size(12.0));
        ui.add_space(space::SM);

        ui.label(RichText::new(desc).color(color::TEXT_MUTED).monospace().size(11.0));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Progress bar
            let fill = if frac > 0.8 {
                color::ACCENT_RED.linear_multiply(0.6)
            } else if frac > 0.5 {
                color::ACCENT_YELLOW.linear_multiply(0.5)
            } else {
                color::ACCENT_PURPLE.linear_multiply(0.4)
            };
            ui.add(egui::ProgressBar::new(frac as f32).desired_width(56.0).fill(fill).show_percentage());

            ui.add_space(space::SM);
            ui.label(
                RichText::new(format!("{:.2}{}", *value, suffix))
                    .color(color::TEXT_PRIMARY)
                    .monospace()
                    .size(12.0),
            );
            ui.add(
                egui::DragValue::new(value)
                    .speed(_default * 0.1)
                    .range(range)
                    .suffix(""),
            );
        });
    });
}

// ── per-gate noise panel ────────────────────────────────────────────────

fn per_gate_panel(ui: &mut egui::Ui, cfg: &mut NoiseConfig) {
    cfg.ensure_default_gates();

    let gate_keys: Vec<String> = cfg.noise_per_gate.keys().cloned().collect();
    if gate_keys.is_empty() {
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("no gates configured").color(color::TEXT_DIM).monospace());
        });
        return;
    }

    egui::Frame::NONE
        .fill(color::BG_ELEVATED)
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::same(space::MD as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());

            // Column headers
            ui.horizontal(|ui| {
                ui.label(RichText::new("gate").color(color::TEXT_MUTED).monospace().size(11.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("damping γ").color(color::TEXT_MUTED).monospace().size(11.0));
                    ui.add_space(64.0);
                    ui.label(RichText::new("depol p").color(color::TEXT_MUTED).monospace().size(11.0));
                });
            });
            ui.add_space(space::XS);
            hairline(ui);
            ui.add_space(space::XS);

            for (i, name) in gate_keys.iter().enumerate() {
                if i > 0 {
                    ui.add_space(2.0);
                }
                if let Some(params) = cfg.noise_per_gate.get_mut(name) {
                    ui.horizontal(|ui| {
                        // Gate name badge
                        let badge_bg = color::ACCENT_YELLOW.linear_multiply(0.15);
                        let badge_size = Vec2::new(32.0, 18.0);
                        let (badge_rect, _) = ui.allocate_exact_size(badge_size, egui::Sense::hover());
                        ui.painter().rect_filled(
                            badge_rect,
                            CornerRadius::same(3),
                            badge_bg,
                        );
                        ui.painter().text(
                            badge_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            name,
                            egui::FontId::monospace(11.0),
                            color::ACCENT_YELLOW,
                        );

                        ui.add_space(space::SM);

                        // Depol mini bar + value
                        let depol_frac = (params.depolarizing_prob / 0.5).clamp(0.0, 1.0);
                        let depol_color = if depol_frac < 0.1 { color::ACCENT_GREEN } else if depol_frac < 0.3 { color::ACCENT_YELLOW } else { color::ACCENT_RED };
                        mini_bar(ui, depol_frac, depol_color);

                        ui.add(
                            egui::DragValue::new(&mut params.depolarizing_prob)
                                .speed(0.0001)
                                .range(0.0..=0.5)
                                .suffix("")
                                .max_decimals(4),
                        );

                        ui.add_space(space::MD);

                        // Damping mini bar + value
                        let damp_frac = (params.damping_gamma / 0.5).clamp(0.0, 1.0);
                        let damp_color = if damp_frac < 0.1 { color::ACCENT_GREEN } else if damp_frac < 0.3 { color::ACCENT_YELLOW } else { color::ACCENT_RED };
                        mini_bar(ui, damp_frac, damp_color);

                        ui.add(
                            egui::DragValue::new(&mut params.damping_gamma)
                                .speed(0.0001)
                                .range(0.0..=0.5)
                                .suffix("")
                                .max_decimals(4),
                        );
                    });
                }
            }
        });
}

fn mini_bar(ui: &mut egui::Ui, frac: f32, color: egui::Color32) {
    let bar_w = 24.0;
    let bar_h = 6.0;
    let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(bar_w, bar_h), egui::Sense::hover());
    ui.painter().rect_filled(bar_rect, CornerRadius::same(1), crate::theme::color::BG);
    if frac > 0.0 {
        let fill_w = bar_w * frac;
        let fill_rect = egui::Rect::from_min_size(bar_rect.min, Vec2::new(fill_w, bar_h));
        ui.painter().rect_filled(fill_rect, CornerRadius::same(1), color);
    }
}

// ── calibration section ─────────────────────────────────────────────────

fn calibration_section(ui: &mut egui::Ui, cfg: &mut NoiseConfig) {
    egui::Frame::NONE
        .fill(color::BG_ELEVATED)
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::same(space::MD as i8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("source").color(color::TEXT_MUTED).monospace());
                ui.add_space(space::SM);
                egui::ComboBox::new("cal_src", "")
                    .selected_text(
                        RichText::new(cfg.calibration_source.label())
                            .monospace()
                            .color(color::TEXT_PRIMARY)
                            .text()
                            .to_string(),
                    )
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

                if cfg.calibration_source != CalibrationSource::None {
                    ui.add_space(space::SM);
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
                        ui.add_space(space::SM);
                        ui.label(RichText::new("●").color(color::ACCENT_GREEN));
                        ui.label(
                            RichText::new(&cfg.calibration_device_name)
                                .color(color::ACCENT_GREEN)
                                .monospace()
                                .size(12.0),
                        );
                    }
                }
            });
        });
}

// ── helpers ────────────────────────────────────────────────────────────

fn section_header(ui: &mut egui::Ui, label: &str) {
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new(label)
                .color(color::TEXT_MUTED)
                .monospace()
                .size(11.0),
        );
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
