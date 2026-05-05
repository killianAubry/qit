// Reusable grid + snap utility.
//
// `Grid` is intentionally framework-agnostic: it speaks `egui::Pos2`/`Rect`
// only because that's the geometry type already in use, but it has no notion
// of "circuit" or "qubit". Any panel that needs cell-aligned layout (today:
// the circuit visualizer; tomorrow: layout helpers, gate palettes, …) can
// reuse this without dragging in domain types.

use egui::{Pos2, Rect, Vec2};

/// A 2D grid of equal-sized cells anchored at `origin`.
#[derive(Clone, Copy, Debug)]
pub struct Grid {
    pub origin: Pos2,
    pub cell: Vec2,
    pub cols: usize,
    pub rows: usize,
}

// Many helpers (snap, cell_rect, full_rect, …) are intentionally part of the
// public grid API — kept around so future widgets can reuse the same coord
// system without each re-deriving its math.
#[allow(dead_code)]
impl Grid {
    pub fn new(origin: Pos2, cell: Vec2, cols: usize, rows: usize) -> Self {
        Self { origin, cell, cols, rows }
    }

    /// Center of the cell at `(col, row)`.
    pub fn cell_center(&self, col: usize, row: usize) -> Pos2 {
        self.origin
            + Vec2::new(
                (col as f32 + 0.5) * self.cell.x,
                (row as f32 + 0.5) * self.cell.y,
            )
    }

    /// Bounding rectangle for cell `(col, row)`.
    pub fn cell_rect(&self, col: usize, row: usize) -> Rect {
        let top_left = self.origin
            + Vec2::new(col as f32 * self.cell.x, row as f32 * self.cell.y);
        Rect::from_min_size(top_left, self.cell)
    }

    /// Vertical center of `row` (used to draw qubit wires).
    pub fn row_y(&self, row: usize) -> f32 {
        self.origin.y + (row as f32 + 0.5) * self.cell.y
    }

    /// Horizontal center of `col`.
    #[allow(dead_code)]
    pub fn col_x(&self, col: usize) -> f32 {
        self.origin.x + (col as f32 + 0.5) * self.cell.x
    }

    /// Full pixel size of the grid.
    pub fn total_size(&self) -> Vec2 {
        Vec2::new(self.cols as f32 * self.cell.x, self.rows as f32 * self.cell.y)
    }

    /// Bounding rectangle of the entire grid.
    pub fn full_rect(&self) -> Rect {
        Rect::from_min_size(self.origin, self.total_size())
    }

    /// Snap a screen position to the cell containing it. Returns `None` if
    /// the point is outside the grid.
    pub fn snap(&self, pos: Pos2) -> Option<(usize, usize)> {
        if !self.full_rect().contains(pos) {
            return None;
        }
        let local = pos - self.origin;
        let col = (local.x / self.cell.x).floor() as i32;
        let row = (local.y / self.cell.y).floor() as i32;
        if col < 0 || row < 0 || col >= self.cols as i32 || row >= self.rows as i32 {
            return None;
        }
        Some((col as usize, row as usize))
    }
}
