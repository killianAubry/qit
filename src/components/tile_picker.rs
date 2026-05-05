// Tile picker overlay (Cmd+T).
//
// Tiny modal listing the available views as numbered options. The user can
// click one or press 1..5 to spawn a new tile next to the focused one. The
// split direction is chosen automatically from the focused tile's aspect
// ratio (auto_split_dir) so wide tiles split horizontally and tall tiles
// split vertically.

use egui::{Align2, CornerRadius, Key, RichText};

use crate::state::{AppState, StatusKind};
use crate::theme::{color, space};
use crate::tiling::{auto_split_dir, ViewKind};

/// Returns true if the picker is currently open.
pub fn show(ctx: &egui::Context, state: &mut AppState, focused_rect: egui::Rect) -> bool {
    if !state.ui.tile_picker_open {
        return false;
    }

    let screen = ctx.content_rect();

    egui::Area::new(egui::Id::new("tile_picker"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_TOP, egui::vec2(0.0, screen.height() * 0.22))
        .show(ctx, |ui| {
            let frame = egui::Frame::NONE
                .fill(color::BG_PANEL)
                .corner_radius(CornerRadius::same(6))
                .inner_margin(egui::Margin::same(space::LG as i8));

            frame.show(ui, |ui| {
                ui.set_width(380.0);
                ui.label(
                    RichText::new("open tile")
                        .monospace()
                        .color(color::TEXT_MUTED),
                );
                ui.add_space(space::SM);

                for (i, &view) in ViewKind::picker_options().iter().enumerate() {
                    let key_label = format!("⌘{}", i + 1);
                    let row = ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{:>2}", i + 1))
                                .monospace()
                                .color(color::ACCENT_YELLOW),
                        );
                        ui.label(
                            RichText::new(view.label())
                                .monospace()
                                .color(color::TEXT_PRIMARY),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(key_label)
                                    .monospace()
                                    .color(color::TEXT_DIM),
                            );
                        });
                    });
                    if row.response.clicked()
                        || row.response.interact(egui::Sense::click()).clicked()
                    {
                        spawn(state, view, focused_rect);
                    }
                }

                ui.add_space(space::SM);
                ui.label(
                    RichText::new("press 1..6, esc to cancel")
                        .monospace()
                        .color(color::TEXT_DIM),
                );
            });
        });

    // Number-key shortcuts
    let pressed = ctx.input(|i| {
        for (n, key) in [
            (1usize, Key::Num1),
            (2, Key::Num2),
            (3, Key::Num3),
            (4, Key::Num4),
            (5, Key::Num5),
            (6, Key::Num6),
        ] {
            if i.key_pressed(key) {
                return Some(n);
            }
        }
        None
    });
    if let Some(n) = pressed {
        if let Some(&view) = ViewKind::picker_options().get(n - 1) {
            spawn(state, view, focused_rect);
        }
    }

    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        state.ui.tile_picker_open = false;
    }

    true
}

fn spawn(state: &mut AppState, view: ViewKind, focused_rect: egui::Rect) {
    let dir = auto_split_dir(focused_rect.size());
    if let Some(id) = state.tiles.split_focused(state.focused_tile, view, dir) {
        state.focused_tile = id;
        state
            .ui
            .flash(format!("opened {} tile", view.label()), StatusKind::Ok);
    } else {
        state.ui.flash("could not split focused tile", StatusKind::Err);
    }
    state.ui.tile_picker_open = false;
}
