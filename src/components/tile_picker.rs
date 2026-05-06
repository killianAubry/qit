// Tile picker overlay (Cmd+T).
//
// Modal with a search input at the top. As the user types, the list of
// available views is filtered by partial match against each label. The
// first match is always selected so pressing Enter opens it immediately.
// Click, Cmd+1..6, and arrow keys also work.

use egui::{Align2, Color32, CornerRadius, Id, Key, RichText, Stroke, StrokeKind};

use crate::state::{AppState, StatusKind};
use crate::theme::{color, space};
use crate::tiling::{auto_split_dir, ViewKind};

pub fn show(ctx: &egui::Context, state: &mut AppState, focused_rect: egui::Rect) -> bool {
    if !state.ui.tile_picker_open {
        return false;
    }

    let screen = ctx.content_rect();
    let options = ViewKind::picker_options();

    // — Filter by typed input —
    let needle = state.ui.tile_picker_input.to_lowercase();
    let matches: Vec<(usize, &ViewKind)> = options
        .iter()
        .enumerate()
        .filter(|(_, v)| {
            if needle.is_empty() {
                return true;
            }
            v.label().to_lowercase().contains(&needle)
        })
        .collect();

    // Default selected index to 0 (first match).
    let mut selected: usize = 0;

    // — Arrow-key navigation —
    ctx.input(|i| {
        if i.key_pressed(Key::ArrowDown) && selected + 1 < matches.len() {
            selected += 1;
        }
        if i.key_pressed(Key::ArrowUp) && selected > 0 {
            selected -= 1;
        }
    });

    // — Number-key shortcuts (Cmd+1..9 handled in app.rs) —
    let direct = ctx.input(|i| {
        for (n, key) in [
            (1usize, Key::Num1),
            (2, Key::Num2),
            (3, Key::Num3),
            (4, Key::Num4),
            (5, Key::Num5),
            (6, Key::Num6),
            (7, Key::Num7),
            (8, Key::Num8),
            (9, Key::Num9),
        ] {
            if i.key_pressed(key) {
                return Some(n);
            }
        }
        None
    });
    if let Some(n) = direct {
        if let Some(&view) = options.get(n - 1) {
            spawn(state, view, focused_rect);
            return false;
        }
    }

    // — Escape —
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        state.ui.tile_picker_open = false;
        state.ui.tile_picker_input.clear();
        return false;
    }

    // — Render —
    egui::Area::new(Id::new("tile_picker"))
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

                // — Search input —
                let search_resp = ui.add(
                    egui::TextEdit::singleline(&mut state.ui.tile_picker_input)
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace)
                        .hint_text("type to filter, Enter to open…"),
                );
                search_resp.request_focus();

                // Enter → open first match.
                if ui.input(|i| i.key_pressed(Key::Enter)) && !matches.is_empty() {
                    let &view = matches[selected].1;
                    state.ui.tile_picker_input.clear();
                    spawn(state, view, focused_rect);
                    return;
                }

                ui.add_space(space::SM);
                draw_hairline(ui);
                ui.add_space(space::SM);

                if matches.is_empty() {
                    ui.label(
                        RichText::new("no matches")
                            .monospace()
                            .color(color::TEXT_DIM),
                    );
                } else {
                    for (i, (_orig_idx, &view)) in matches.iter().enumerate() {
                        let is_sel = i == selected;
                        let bg = if is_sel {
                            color::BG_ACTIVE
                        } else {
                            Color32::TRANSPARENT
                        };
                        let fg = if is_sel {
                            color::ACCENT_YELLOW
                        } else {
                            color::TEXT_PRIMARY
                        };

                        let row_resp = egui::Frame::NONE
                            .fill(bg)
                            .corner_radius(CornerRadius::same(3))
                            .inner_margin(egui::Margin::symmetric(6, 3))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    let key_label = format!("⌘{}", _orig_idx + 1);
                                    ui.label(
                                        RichText::new(format!("{:>2}", _orig_idx + 1))
                                            .monospace()
                                            .color(fg),
                                    );
                                    ui.label(
                                        RichText::new(view.label())
                                            .monospace()
                                            .color(fg),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                RichText::new(key_label)
                                                    .monospace()
                                                    .color(color::TEXT_DIM),
                                            );
                                        },
                                    );
                                });
                            });

                        if row_resp.response.clicked()
                            || row_resp.response.interact(egui::Sense::click()).clicked()
                        {
                            state.ui.tile_picker_input.clear();
                            spawn(state, view, focused_rect);
                            return;
                        }
                    }
                }

                ui.add_space(space::SM);
                ui.label(
                    RichText::new("type to filter · ↑↓ · enter · esc")
                        .monospace()
                        .color(color::TEXT_DIM),
                );
            });
        });

    true
}

fn draw_hairline(ui: &mut egui::Ui) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(0),
        Stroke::new(1.0, color::GRID_LINE),
        StrokeKind::Inside,
    );
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
