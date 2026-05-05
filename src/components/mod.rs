// Component layer.
//
// Every visible piece of the UI is a free `show` function in its own module.
// Free functions (instead of a `Panel` trait) keep the call sites simple —
// `app.rs` dispatches based on `ViewKind` without juggling boxed trait
// objects. Per-tile state (e.g. bloch yaw/pitch) is threaded through as a
// `&mut LeafTile` where it's needed.

pub mod bloch_3d;
pub mod circuit_visualizer;
pub mod command_palette;
pub mod editor;
pub mod probability_panel;
pub mod state_vector;
pub mod status_bar;
pub mod tile_picker;
pub mod benchmark_panel;

use crate::state::AppState;
use crate::tiling::{LeafTile, ViewKind};

/// Render the body of a tile leaf — the dispatch from `ViewKind` to the
/// concrete `show` function. Keeping it here means `app.rs` doesn't have to
/// know about every component module.
pub fn render_view(view: ViewKind, ui: &mut egui::Ui, state: &mut AppState, leaf: &mut LeafTile) {
    match view {
        ViewKind::Editor => editor::show(ui, state),
        ViewKind::Circuit => circuit_visualizer::show(ui, state),
        ViewKind::Probability => probability_panel::show(ui, state),
        ViewKind::StateVector => state_vector::show(ui, state),
        ViewKind::Bloch => bloch_3d::show(ui, state, leaf),
        ViewKind::Benchmark => benchmark_panel::show(ui, state),
    }
}
