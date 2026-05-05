// Application root.
//
// Owns `AppState` and walks the tile tree on every frame to render leaves
// inside the central panel. The body of `update`:
//
//   1. Sync state (parse + simulate the editor buffer if it changed).
//   2. Handle global keybinds via `ctx.input_mut().consume_key(...)`. We do
//      this *before* drawing any widget so Cmd-shortcuts never reach the
//      editor as character input.
//   3. Draw the top + bottom status strips.
//   4. Render the tile tree in the central panel: for each leaf draw a
//      thin border (accent color when focused), call the right component,
//      and attach a click-to-focus sensor. Invisible split gutters (no
//      divider line) sit on top for drag-to-resize.
//   5. Draw modal overlays (command palette, tile picker) last so they
//      capture pointer + keyboard until dismissed.
//
// All of the actual drawing logic lives in `components/*`. This file only
// decides geometry, focus, and key dispatch.

use egui::{CornerRadius, Id, Key, Modifiers, Pos2, Rect, Sense, Stroke, StrokeKind, UiBuilder};
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use crate::components::{self, command_palette, status_bar, tile_picker};
use crate::state::{AppState, StatusKind};
use crate::theme::{self, color, space};
use crate::tiling::{self, FocusDir, Layout, SplitDir, ViewKind};

pub struct QSimApp {
    state: AppState,
    /// Rect occupied by the tile tree on the previous frame; consulted by
    /// `apply_action` so keybind-driven splits/focus can compute the right
    /// geometry without waiting for `render_tile_tree` to run again.
    last_central_rect: Rect,
}

impl QSimApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install(&cc.egui_ctx);
        Self {
            state: AppState::new(),
            last_central_rect: Rect::NOTHING,
        }
    }
}

impl eframe::App for QSimApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // 1. Keep simulation in sync with the editor buffer.
        self.state.ensure_synced();

        // 2. Global keybinds — consumed before any widget sees them.
        let modal_open = self.state.ui.cmd_palette_open || self.state.ui.tile_picker_open;
        if !modal_open {
            self.handle_global_keys(ctx, frame);
        }

        // 3. Status strips (no buttons by design — keys only).
        egui::TopBottomPanel::top("status_top")
            .exact_height(26.0)
            .frame(panel_chrome())
            .show(ctx, |ui| status_bar::show_top(ui, &mut self.state));

        egui::TopBottomPanel::bottom("status_bottom")
            .exact_height(18.0)
            .frame(panel_chrome())
            .show(ctx, |ui| status_bar::show_bottom(ui, &mut self.state));

        // 4. Central tile tree.
        let mut focused_rect = Rect::NOTHING;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(color::BG)
                    .inner_margin(egui::Margin::same(space::MD as i8)),
            )
            .show(ctx, |ui| {
                focused_rect = self.render_tile_tree(ui);
            });

        // 5. Click-to-focus: any pointer press inside a leaf rect becomes
        // the focused tile, regardless of whether a child widget also
        // received the click.
        if !modal_open {
            let click = ctx.input(|i| {
                i.pointer.primary_clicked().then(|| i.pointer.interact_pos()).flatten()
            });
            if let Some(p) = click {
                let layout = self.state.tiles.layout(self.last_central_rect);
                for (leaf, rect) in &layout.leaves {
                    if rect.contains(p) {
                        self.state.focused_tile = leaf.id;
                        break;
                    }
                }
            }
        }

        // 6. Modal overlays.
        let palette_open = command_palette::show(ctx, &mut self.state, focused_rect);
        let picker_open = tile_picker::show(ctx, &mut self.state, focused_rect);
        let _ = palette_open || picker_open;
    }
}

impl QSimApp {
    /// Walks the tile tree, draws every leaf + split handle, and returns
    /// the rect of the focused leaf so the modal overlays can anchor near it.
    fn render_tile_tree(&mut self, ui: &mut egui::Ui) -> Rect {
        let root_rect = ui.max_rect();
        self.last_central_rect = root_rect;
        let layout = self.state.tiles.layout(root_rect);

        // Snapshot focused id locally so the borrow checker doesn't trip on
        // the mutable `state` borrow inside the leaf loop.
        let focused = self.state.focused_tile;
        let mut focused_rect = Rect::NOTHING;

        // — Leaves —
        // Iterate via index because we need disjoint mutable access to each
        // leaf's per-tile state (bloch yaw/pitch).
        for i in 0..layout.leaves.len() {
            let (leaf_view, rect) = layout.leaves[i].clone();
            let is_focused = leaf_view.id == focused;
            if is_focused {
                focused_rect = rect;
            }
            self.render_leaf(ui, leaf_view.id, leaf_view.view, rect, is_focused);
        }

        // — Split handles (drawn last so they sit on top of borders) —
        self.render_handles(ui, &layout);

        focused_rect
    }

    fn render_leaf(
        &mut self,
        ui: &mut egui::Ui,
        id: tiling::TileId,
        view: ViewKind,
        rect: Rect,
        is_focused: bool,
    ) {
        let inner = rect.shrink(8.0);
        let body = inner.shrink(2.0);

        // Clone the leaf out of the tree so we don't hold a borrow on
        // `state.tiles` while `render_view` mutably borrows `state`.
        // (Per-tile mutations get written back below.)
        let mut leaf_copy = self
            .state
            .tiles
            .find_leaf_mut(id)
            .expect("leaf id was just produced by `layout`")
            .clone();

        ui.scope_builder(UiBuilder::new().max_rect(body), |ui| {
            components::render_view(view, ui, &mut self.state, &mut leaf_copy);
        });

        if let Some(slot) = self.state.tiles.find_leaf_mut(id) {
            *slot = leaf_copy;
        }

        let stroke_color = if is_focused {
            color::ACCENT_YELLOW
        } else {
            color::GRID_LINE
        };
        ui.painter().rect_stroke(
            inner,
            CornerRadius::same(2),
            Stroke::new(1.0, stroke_color),
            StrokeKind::Inside,
        );
    }

    fn render_handles(&mut self, ui: &mut egui::Ui, layout: &Layout) {
        for handle in &layout.handles {
            let resp = ui.interact(
                handle.rect,
                Id::new(("split_handle", handle.path.clone())),
                Sense::click_and_drag(),
            );
            let hot = resp.hovered() || resp.dragged();

            if resp.dragged() {
                let delta = match handle.dir {
                    SplitDir::Horizontal => resp.drag_delta().x,
                    SplitDir::Vertical => resp.drag_delta().y,
                };
                tiling::drag_split(
                    &mut self.state.tiles,
                    &handle.path,
                    delta,
                    handle.parent_along_axis,
                );
            }

            if hot {
                ui.ctx().set_cursor_icon(match handle.dir {
                    SplitDir::Horizontal => egui::CursorIcon::ResizeColumn,
                    SplitDir::Vertical => egui::CursorIcon::ResizeRow,
                });
            }
        }
    }

    fn central_rect(&self, ctx: &egui::Context) -> Rect {
        if self.last_central_rect.area() > 1.0 {
            return self.last_central_rect;
        }
        // First-frame fallback: best-effort guess from the content rect.
        let m = space::MD;
        let r = ctx.content_rect();
        Rect::from_min_max(
            Pos2::new(r.min.x + m, r.min.y + m + 28.0),
            Pos2::new(r.max.x - m, r.max.y - m - 22.0),
        )
    }

    fn handle_global_keys(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        let cmds = ctx.input_mut(|i| {
            let mut out: Vec<KeyAction> = Vec::new();

            if i.consume_key(Modifiers::COMMAND, Key::E) {
                out.push(KeyAction::TogglePalette);
            }
            if i.consume_key(Modifiers::COMMAND, Key::T) {
                out.push(KeyAction::ToggleTilePicker);
            }
            if i.consume_key(Modifiers::COMMAND, Key::W) {
                out.push(KeyAction::CloseFocused);
            }
            if i.consume_key(Modifiers::COMMAND, Key::R) {
                out.push(KeyAction::Run);
            }
            if i.consume_key(Modifiers::COMMAND, Key::S) {
                out.push(KeyAction::Save);
            }
            if i.consume_key(Modifiers::COMMAND, Key::O) {
                out.push(KeyAction::Load);
            }

            // Cycle tile focus (read order matches layout.leaves — depth-first).
            let shift_cmd = Modifiers::COMMAND | Modifiers::SHIFT;
            if i.consume_key(shift_cmd, Key::Tab) {
                out.push(KeyAction::CycleFocus(false));
            } else if i.consume_key(Modifiers::COMMAND, Key::Tab) {
                out.push(KeyAction::CycleFocus(true));
            }

            // Vim-style focus motion (Cmd + H/J/K/L)
            if i.consume_key(Modifiers::COMMAND, Key::H) {
                out.push(KeyAction::Focus(FocusDir::Left));
            }
            if i.consume_key(Modifiers::COMMAND, Key::J) {
                out.push(KeyAction::Focus(FocusDir::Down));
            }
            if i.consume_key(Modifiers::COMMAND, Key::K) {
                out.push(KeyAction::Focus(FocusDir::Up));
            }
            if i.consume_key(Modifiers::COMMAND, Key::L) {
                out.push(KeyAction::Focus(FocusDir::Right));
            }

            // Cmd + Shift + H/J/K/L → split focused tile in that direction
            // and open the picker in the resulting tile.
            let split_shift_cmd = Modifiers::COMMAND | Modifiers::SHIFT;
            if i.consume_key(split_shift_cmd, Key::H) {
                out.push(KeyAction::SplitDir(SplitDir::Horizontal, false));
            }
            if i.consume_key(split_shift_cmd, Key::L) {
                out.push(KeyAction::SplitDir(SplitDir::Horizontal, true));
            }
            if i.consume_key(split_shift_cmd, Key::K) {
                out.push(KeyAction::SplitDir(SplitDir::Vertical, false));
            }
            if i.consume_key(split_shift_cmd, Key::J) {
                out.push(KeyAction::SplitDir(SplitDir::Vertical, true));
            }

        // Cmd + 1..6 → open that view directly (skips the picker).
        for (n, key) in [
            (1usize, Key::Num1),
            (2, Key::Num2),
            (3, Key::Num3),
            (4, Key::Num4),
            (5, Key::Num5),
            (6, Key::Num6),
        ] {
                if i.consume_key(Modifiers::COMMAND, key) {
                    if let Some(&v) = ViewKind::picker_options().get(n - 1) {
                        out.push(KeyAction::OpenView(v));
                    }
                }
            }

            out
        });

        for cmd in cmds {
            self.apply_action(ctx, frame, cmd);
        }
    }

    fn apply_action(&mut self, ctx: &egui::Context, frame: &eframe::Frame, action: KeyAction) {
        match action {
            KeyAction::TogglePalette => {
                self.state.ui.cmd_palette_open = !self.state.ui.cmd_palette_open;
                if !self.state.ui.cmd_palette_open {
                    self.state.ui.cmd_palette_input.clear();
                }
            }
            KeyAction::ToggleTilePicker => {
                self.state.ui.tile_picker_open = !self.state.ui.tile_picker_open;
                if !self.state.ui.tile_picker_open {
                    self.state.ui.tile_picker_input.clear();
                }
            }
            KeyAction::CloseFocused => {
                match self.state.tiles.close_focused(self.state.focused_tile) {
                    tiling::CloseResult::Closed(id) => {
                        self.state.focused_tile = id;
                        self.state.ui.flash("tile closed", StatusKind::Ok);
                    }
                    tiling::CloseResult::WasOnlyLeaf => {
                        self.state.ui.flash(
                            "can't close — last tile (try ⌘T to open another)",
                            StatusKind::Err,
                        );
                    }
                    tiling::CloseResult::NotFound => {}
                }
            }
            KeyAction::Run => match self.state.rerun() {
                Ok(()) => {
                    let label = self.state.simulator.label();
                    self.state
                        .ui
                        .flash(format!("run ({label}) ok"), StatusKind::Ok);
                }
                Err(e) => self.state.ui.flash(e, StatusKind::Err),
            },
            KeyAction::Save => match self.state.save_circuit_file() {
                Ok(()) => {
                    let p = self.state.circuit_file_path();
                    self.state
                        .ui
                        .flash(format!("saved {}", p.display()), StatusKind::Ok);
                }
                Err(e) => self.state.ui.flash(e, StatusKind::Err),
            },
            KeyAction::Load => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    // Clear modifiers to prevent keys getting stuck
                    ctx.input_mut(|i| i.modifiers = egui::Modifiers::default());
                    
                    let path = rfd::FileDialog::new()
                        .set_title("Open OpenQASM file")
                        .add_filter("OpenQASM", &["qasm"])
                        .set_directory(self.state.workspace_dir.clone())
                        .pick_file();
                        
                    if let Some(path) = path {
                        match std::fs::read_to_string(&path) {
                            Ok(text) => match self.state.load_editor_text_from_path(path.clone(), text) {
                                Ok(()) => {
                                    self.state
                                        .ui
                                        .flash(format!("loaded {}", path.display()), StatusKind::Ok);
                                }
                                Err(e) => self.state.ui.flash(e, StatusKind::Err),
                            },
                            Err(e) => self
                                .state
                                .ui
                                .flash(format!("read {}: {e}", path.display()), StatusKind::Err),
                        }
                    }
                }
                #[cfg(target_arch = "wasm32")]
                self.state
                    .ui
                    .flash("open file: desktop builds only", StatusKind::Err);
            }
            KeyAction::Focus(dir) => {
                let layout = self.state.tiles.layout(self.central_rect(ctx));
                if let Some(id) =
                    tiling::focus_neighbor(&layout, self.state.focused_tile, dir)
                {
                    self.state.focused_tile = id;
                }
            }
            KeyAction::CycleFocus(forward) => {
                let layout = self.state.tiles.layout(self.central_rect(ctx));
                if let Some(id) =
                    tiling::focus_cycle(&layout, self.state.focused_tile, forward)
                {
                    self.state.focused_tile = id;
                }
            }
            KeyAction::SplitDir(dir, _put_new_after) => {
                // Open the picker; the next view choice will use this dir.
                // (The picker today calls auto_split_dir; an explicit override
                // could thread `dir` into `state`.)
                self.state.ui.tile_picker_open = true;
                let _ = dir;
            }
            KeyAction::OpenView(v) => {
                let layout = self.state.tiles.layout(self.central_rect(ctx));
                let focused_rect = layout
                    .leaves
                    .iter()
                    .find(|(l, _)| l.id == self.state.focused_tile)
                    .map(|(_, r)| *r)
                    .unwrap_or_else(|| self.central_rect(ctx));
                let dir = tiling::auto_split_dir(focused_rect.size());
                if let Some(id) = self.state.tiles.split_focused(
                    self.state.focused_tile,
                    v,
                    dir,
                ) {
                    self.state.focused_tile = id;
                    self.state
                        .ui
                        .flash(format!("opened {}", v.label()), StatusKind::Ok);
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum KeyAction {
    TogglePalette,
    ToggleTilePicker,
    CloseFocused,
    Run,
    Save,
    Load,
    Focus(FocusDir),
    /// Next / previous tile in layout order (`⌘Tab` / `⌘⇧Tab`).
    CycleFocus(bool),
    SplitDir(SplitDir, bool),
    OpenView(ViewKind),
}

fn panel_chrome() -> egui::Frame {
    egui::Frame::NONE
        .fill(color::BG)
        .inner_margin(egui::Margin::symmetric(space::MD as i8, 3))
}
