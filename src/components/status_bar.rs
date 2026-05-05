// Status bar — minimal chrome only.
//
// Top: workspace folder (opens native picker, sets save/load directory) +
// themed simulator ComboBox + TurboSpin compression selector when relevant.
// Bottom: transient `status_message` only.

use egui::{CornerRadius, RichText, Sense, Stroke, StrokeKind, Vec2};

use crate::state::{AppState, SimulatorKind, StatusKind, TurboSpinCompression};
use crate::theme::{color, space};
use crate::workspace;

pub fn show_top(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.add_space(space::SM);

        let resp = folder_glyph(ui)
            .on_hover_text(workspace_tooltip(state));

        if resp.clicked() {
            workspace::pick_workspace_folder(state);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(space::SM);
            combo_sim(ui, state);
            if state.simulator == SimulatorKind::TurboSpin {
                ui.add_space(space::SM);
                combo_turbospin_compression(ui, state);
            }
        });
    });
}

fn workspace_tooltip(state: &AppState) -> String {
    format!(
        "{}\n{}\n⌘S save · ⌘O open .qasm…",
        state.workspace_dir.display(),
        state.circuit_file_path().display(),
    )
}

/// Tab + body outline — reads as a folder at 14×11 px, stroke-only; click
/// opens the workspace directory picker.
fn folder_glyph(ui: &mut egui::Ui) -> egui::Response {
    let size = Vec2::new(14.0, 11.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let p = ui.painter_at(rect);

    let stroke_color = if resp.hovered() {
        color::ACCENT_YELLOW
    } else {
        color::TEXT_MUTED
    };
    let stroke = Stroke::new(1.0, stroke_color);
    let skind = StrokeKind::Inside;

    let tab = egui::Rect::from_min_max(rect.min, rect.min + Vec2::new(7.0, 4.0));
    p.rect_stroke(tab, CornerRadius::same(1), stroke, skind);

    let body = egui::Rect::from_min_max(rect.min + Vec2::new(0.0, 3.0), rect.max);
    p.rect_stroke(body, CornerRadius::same(2), stroke, skind);

    resp
}

fn combo_sim(ui: &mut egui::Ui, state: &mut AppState) {
    ui.scope(|ui| {
        style_combo_widgets(ui);

        let prev = state.simulator;

        egui::ComboBox::from_id_salt("top_sim")
            .width(152.0)
            .selected_text(
                RichText::new(state.simulator.label())
                    .monospace()
                    .color(color::TEXT_PRIMARY),
            )
            .show_ui(ui, |ui| {
                for &kind in SimulatorKind::ALL {
                    ui.selectable_value(&mut state.simulator, kind, kind.label());
                }
            });

        if state.simulator != prev {
            on_simulator_changed(state, prev);
        }
    });
}

fn combo_turbospin_compression(ui: &mut egui::Ui, state: &mut AppState) {
    ui.scope(|ui| {
        style_combo_widgets(ui);

        let prev = state.turbospin_compression;
        let selected = format!("ts {}", state.turbospin_compression.label());

        let resp = egui::ComboBox::from_id_salt("top_turbospin_compression")
            .width(110.0)
            .selected_text(
                RichText::new(selected)
                    .monospace()
                    .color(color::TEXT_PRIMARY),
            )
            .show_ui(ui, |ui| {
                for &choice in TurboSpinCompression::ALL {
                    ui.selectable_value(
                        &mut state.turbospin_compression,
                        choice,
                        choice.label(),
                    );
                }
            })
            .response
            .on_hover_text(
                "TurboSpin compression\nexact = --comp-bit 0 (raw Spinoza)\n1..8 = BACQS hybrid path (--comp-bit N)",
            );

        let _ = resp;

        if state.turbospin_compression != prev {
            state.ui.flash(
                format!(
                    "turbospin compression: {}  ⌘R to rerun",
                    state.turbospin_compression.label()
                ),
                StatusKind::Info,
            );
        }
    });
}

fn style_combo_widgets(ui: &mut egui::Ui) {
    let w = &mut ui.style_mut().visuals.widgets;
    w.inactive.bg_fill = color::BG_ELEVATED;
    w.inactive.weak_bg_fill = color::BG_ELEVATED;
    w.inactive.fg_stroke = Stroke::new(1.0, color::TEXT_PRIMARY);
    w.inactive.bg_stroke = Stroke::new(1.0, color::GRID_LINE);

    w.hovered.bg_fill = color::BG_HOVER;
    w.hovered.weak_bg_fill = color::BG_HOVER;
    w.hovered.fg_stroke = Stroke::new(1.0, color::ACCENT_YELLOW);
    w.hovered.bg_stroke = Stroke::new(1.0, color::ACCENT_YELLOW.linear_multiply(0.35));

    w.active.bg_fill = color::BG_ACTIVE;
    w.active.weak_bg_fill = color::BG_ACTIVE;
    w.active.fg_stroke = Stroke::new(1.0, color::TEXT_PRIMARY);
    w.active.bg_stroke = Stroke::new(1.0, color::ACCENT_YELLOW.linear_multiply(0.5));

    w.open.bg_fill = color::BG_ELEVATED;
    w.open.fg_stroke = Stroke::new(1.0, color::TEXT_PRIMARY);

    let n = &mut ui.style_mut().visuals.widgets.noninteractive;
    n.bg_fill = color::BG_PANEL;
    n.weak_bg_fill = color::BG_PANEL;
    n.fg_stroke = Stroke::new(1.0, color::TEXT_PRIMARY);
}

/// When switching simulators, seed from the new mode's template only if the
/// buffer still matches the *previous* mode's starter template. Empty buffers
/// stay empty until the user opens a file or runs `:reset`.
fn on_simulator_changed(state: &mut AppState, prev: SimulatorKind) {
    let was_template = state.editor_text == prev.default_template();
    if was_template {
        state.editor_text = state.simulator.default_template().to_string();
    }
    state.ensure_synced();

    let s = state.simulator;
    let suffix = if was_template {
        " (template loaded)"
    } else {
        ""
    };
    state.ui.flash(
        format!(
            "{} → .{} files{}{}  ⌘R to run",
            s.label(),
            s.circuit_extension(),
            suffix,
            if s == SimulatorKind::TurboSpin {
                " · use `ts …` to set compression"
            } else {
                ""
            }
        ),
        StatusKind::Info,
    );
}

pub fn show_bottom(ui: &mut egui::Ui, state: &mut AppState) {
    if let Some((msg, kind)) = state.ui.status_message.as_ref() {
        let c = match kind {
            StatusKind::Info => color::TEXT_MUTED,
            StatusKind::Ok => color::ACCENT_YELLOW,
            StatusKind::Err => color::ACCENT_RED,
        };
        ui.horizontal(|ui| {
            ui.add_space(space::SM);
            ui.label(RichText::new(msg.as_str()).color(c).monospace().size(11.0));
        });
    } else {
        ui.allocate_space(Vec2::new(ui.available_width(), 1.0));
    }
}
