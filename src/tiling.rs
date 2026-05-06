// Window-tile manager.
//
// A binary tree where each leaf is a `View` (editor / circuit / probability /
// state vector / bloch). Splits carry a direction and a 0..1 ratio that the
// user can drag.
//
//   • `Tile::split_focused`   — replace the focused leaf with a Split.
//   • `Tile::close_focused`   — replace its parent split with the sibling.
//   • `Tile::layout`          — flatten into `(LeafTile, Rect, Path)` triples
//                                 so `app.rs` can render and dispatch input
//                                 without recursing through the tree itself.
//   • `Tile::focus_neighbor`  — find the nearest leaf in a direction by
//                                 comparing flat layout rects.
//
// Each leaf carries enough per-tile presentation state (Bloch yaw/pitch,
// scroll offsets, …) that splitting a tile never disturbs another tile.

use egui::{Pos2, Rect, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TileId(pub u64);

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl TileId {
    pub fn fresh() -> Self {
        Self(NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewKind {
    Editor,
    Circuit,
    Probability,
    StateVector,
    Bloch,
    Noise,
    Fidelity,
    Entanglement,
    DensityMatrix,
}

impl ViewKind {
    pub fn label(self) -> &'static str {
        match self {
            ViewKind::Editor => "editor",
            ViewKind::Circuit => "circuit",
            ViewKind::Probability => "probability",
            ViewKind::StateVector => "state vector",
            ViewKind::Bloch => "bloch 3d",
            ViewKind::Noise => "noise",
            ViewKind::Fidelity => "fidelity",
            ViewKind::Entanglement => "entanglement",
            ViewKind::DensityMatrix => "density matrix",
        }
    }

    pub fn picker_options() -> &'static [ViewKind] {
        &[
            ViewKind::Editor,
            ViewKind::Circuit,
            ViewKind::Probability,
            ViewKind::StateVector,
            ViewKind::Bloch,
            ViewKind::Noise,
            ViewKind::Fidelity,
            ViewKind::Entanglement,
            ViewKind::DensityMatrix,
        ]
    }
}

#[derive(Clone, Debug)]
pub struct LeafTile {
    pub id: TileId,
    pub view: ViewKind,
    /// 3D bloch view rotation (yaw, pitch). Persisted per-tile so two bloch
    /// tiles can show different angles.
    pub bloch_yaw: f32,
    pub bloch_pitch: f32,
}

impl LeafTile {
    pub fn new(view: ViewKind) -> Self {
        Self {
            id: TileId::fresh(),
            view,
            bloch_yaw: 0.6,
            bloch_pitch: -0.35,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitDir {
    /// Children stacked left|right.
    Horizontal,
    /// Children stacked top/bottom.
    Vertical,
}

#[derive(Clone, Debug)]
pub struct SplitTile {
    pub dir: SplitDir,
    pub ratio: f32,
    pub a: Box<Tile>,
    pub b: Box<Tile>,
}

#[derive(Clone, Debug)]
pub enum Tile {
    Leaf(LeafTile),
    Split(SplitTile),
}

impl Tile {
    pub fn leaf(view: ViewKind) -> Self {
        Tile::Leaf(LeafTile::new(view))
    }

    /// Replace the leaf with id `target` by a new Split. The new view becomes
    /// the second child (right or bottom) so the existing focus stays put.
    pub fn split_focused(&mut self, target: TileId, view: ViewKind, dir: SplitDir) -> Option<TileId> {
        let new_id;
        match self {
            Tile::Leaf(leaf) if leaf.id == target => {
                let existing = std::mem::replace(leaf, LeafTile::new(ViewKind::Editor));
                let new_leaf = LeafTile::new(view);
                new_id = new_leaf.id;
                *self = Tile::Split(SplitTile {
                    dir,
                    ratio: 0.5,
                    a: Box::new(Tile::Leaf(existing)),
                    b: Box::new(Tile::Leaf(new_leaf)),
                });
            }
            Tile::Split(s) => {
                if let Some(id) = s.a.split_focused(target, view, dir) {
                    new_id = id;
                } else if let Some(id) = s.b.split_focused(target, view, dir) {
                    new_id = id;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
        Some(new_id)
    }

    /// Remove the leaf with id `target`. If it was one half of a split, the
    /// parent collapses into the surviving sibling. Returns the id of a leaf
    /// that should receive focus next, or `None` if the closed tile was the
    /// only one in the tree.
    pub fn close_focused(&mut self, target: TileId) -> CloseResult {
        match self {
            Tile::Leaf(leaf) if leaf.id == target => CloseResult::WasOnlyLeaf,
            Tile::Leaf(_) => CloseResult::NotFound,
            Tile::Split(_) => {
                let take_self = std::mem::replace(self, Tile::Leaf(LeafTile::new(ViewKind::Editor)));
                let Tile::Split(SplitTile { dir, ratio, a, b }) = take_self else {
                    unreachable!()
                };
                // Try to close in `a` first.
                let mut a = *a;
                let mut b = *b;
                match a.close_focused(target) {
                    CloseResult::WasOnlyLeaf => {
                        *self = b;
                        CloseResult::Closed(self.first_leaf().id)
                    }
                    CloseResult::Closed(id) => {
                        *self = Tile::Split(SplitTile {
                            dir,
                            ratio,
                            a: Box::new(a),
                            b: Box::new(b),
                        });
                        CloseResult::Closed(id)
                    }
                    CloseResult::NotFound => match b.close_focused(target) {
                        CloseResult::WasOnlyLeaf => {
                            *self = a;
                            CloseResult::Closed(self.first_leaf().id)
                        }
                        CloseResult::Closed(id) => {
                            *self = Tile::Split(SplitTile {
                                dir,
                                ratio,
                                a: Box::new(a),
                                b: Box::new(b),
                            });
                            CloseResult::Closed(id)
                        }
                        CloseResult::NotFound => {
                            *self = Tile::Split(SplitTile {
                                dir,
                                ratio,
                                a: Box::new(a),
                                b: Box::new(b),
                            });
                            CloseResult::NotFound
                        }
                    },
                }
            }
        }
    }

    /// Return the first leaf encountered by depth-first traversal.
    pub fn first_leaf(&self) -> &LeafTile {
        match self {
            Tile::Leaf(l) => l,
            Tile::Split(s) => s.a.first_leaf(),
        }
    }

    #[allow(dead_code)]
    pub fn first_leaf_mut(&mut self) -> &mut LeafTile {
        match self {
            Tile::Leaf(l) => l,
            Tile::Split(s) => s.a.first_leaf_mut(),
        }
    }

    pub fn find_leaf_mut(&mut self, target: TileId) -> Option<&mut LeafTile> {
        match self {
            Tile::Leaf(l) if l.id == target => Some(l),
            Tile::Leaf(_) => None,
            Tile::Split(s) => s.a.find_leaf_mut(target).or_else(|| s.b.find_leaf_mut(target)),
        }
    }

    /// Flatten the tree into a list of leaves with their pixel rects, plus
    /// a parallel list of split-handle rects (for drag-resize).
    pub fn layout(&self, rect: Rect) -> Layout {
        let mut layout = Layout::default();
        layout_recursive(self, rect, Vec::new(), &mut layout);
        layout
    }
}

#[must_use]
pub enum CloseResult {
    /// Closed and `id` should now receive focus.
    Closed(TileId),
    /// The tile was the only leaf in the tree — caller may want to keep it
    /// open / replace its content rather than really destroying it.
    WasOnlyLeaf,
    NotFound,
}

#[derive(Default, Debug, Clone)]
pub struct Layout {
    pub leaves: Vec<(LeafView, Rect)>,
    pub handles: Vec<SplitHandle>,
}

#[derive(Clone, Debug)]
pub struct LeafView {
    pub id: TileId,
    pub view: ViewKind,
}

#[derive(Clone, Debug)]
pub struct SplitHandle {
    /// Path from the tree root to the split being dragged: each step is
    /// 0 (left/top child) or 1 (right/bottom child).
    pub path: Vec<u8>,
    pub dir: SplitDir,
    pub rect: Rect,
    /// Size of the parent split's rect along its split axis. Used to
    /// translate a pixel drag delta into a ratio change.
    pub parent_along_axis: f32,
}

const HANDLE_THICKNESS: f32 = 4.0;

fn layout_recursive(tile: &Tile, rect: Rect, path: Vec<u8>, out: &mut Layout) {
    match tile {
        Tile::Leaf(l) => {
            out.leaves.push((LeafView { id: l.id, view: l.view }, rect));
        }
        Tile::Split(s) => {
            let (ra, rb, handle) = split_rect(rect, s.dir, s.ratio);
            let along = match s.dir {
                SplitDir::Horizontal => rect.width(),
                SplitDir::Vertical => rect.height(),
            };
            out.handles.push(SplitHandle {
                path: path.clone(),
                dir: s.dir,
                rect: handle,
                parent_along_axis: along,
            });
            let mut path_a = path.clone();
            path_a.push(0);
            layout_recursive(&s.a, ra, path_a, out);
            let mut path_b = path;
            path_b.push(1);
            layout_recursive(&s.b, rb, path_b, out);
        }
    }
}

/// Apply a drag at `path` with delta `delta_px`. Returns true if anything
/// changed. Path is the same one carried by a `SplitHandle`.
pub fn drag_split(root: &mut Tile, path: &[u8], delta_px: f32, available_along_axis: f32) -> bool {
    if available_along_axis <= 1.0 {
        return false;
    }
    let mut node = root;
    for step in path {
        let Tile::Split(s) = node else { return false };
        node = match step {
            0 => &mut *s.a,
            _ => &mut *s.b,
        };
    }
    let Tile::Split(s) = node else { return false };
    s.ratio = (s.ratio + delta_px / available_along_axis).clamp(0.1, 0.9);
    true
}

fn split_rect(rect: Rect, dir: SplitDir, ratio: f32) -> (Rect, Rect, Rect) {
    let r = ratio.clamp(0.1, 0.9);
    match dir {
        SplitDir::Horizontal => {
            let mid = rect.min.x + rect.width() * r;
            let left = Rect::from_min_max(rect.min, Pos2::new(mid - HANDLE_THICKNESS * 0.5, rect.max.y));
            let right = Rect::from_min_max(Pos2::new(mid + HANDLE_THICKNESS * 0.5, rect.min.y), rect.max);
            let handle = Rect::from_min_max(
                Pos2::new(mid - HANDLE_THICKNESS * 0.5, rect.min.y),
                Pos2::new(mid + HANDLE_THICKNESS * 0.5, rect.max.y),
            );
            (left, right, handle)
        }
        SplitDir::Vertical => {
            let mid = rect.min.y + rect.height() * r;
            let top = Rect::from_min_max(rect.min, Pos2::new(rect.max.x, mid - HANDLE_THICKNESS * 0.5));
            let bot = Rect::from_min_max(Pos2::new(rect.min.x, mid + HANDLE_THICKNESS * 0.5), rect.max);
            let handle = Rect::from_min_max(
                Pos2::new(rect.min.x, mid - HANDLE_THICKNESS * 0.5),
                Pos2::new(rect.max.x, mid + HANDLE_THICKNESS * 0.5),
            );
            (top, bot, handle)
        }
    }
}

/// Pick an appropriate split direction for a tile of the given size:
/// split horizontally (creating a side-by-side layout) when the tile is
/// noticeably wider than tall, vertically otherwise. Keeps newly-opened
/// tiles roughly square.
pub fn auto_split_dir(tile_size: Vec2) -> SplitDir {
    if tile_size.x >= tile_size.y * 1.2 {
        SplitDir::Horizontal
    } else {
        SplitDir::Vertical
    }
}

#[derive(Clone, Copy, Debug)]
pub enum FocusDir {
    Left,
    Right,
    Up,
    Down,
}

/// Find the nearest leaf in the requested direction from `from`.
/// Uses the layout rects to pick the best candidate by directional overlap +
/// distance — works for arbitrary tree shapes.
pub fn focus_neighbor(layout: &Layout, from: TileId, dir: FocusDir) -> Option<TileId> {
    let from_rect = layout.leaves.iter().find(|(l, _)| l.id == from).map(|(_, r)| *r)?;
    let from_center = from_rect.center();

    let mut best: Option<(f32, TileId)> = None;
    for (leaf, rect) in &layout.leaves {
        if leaf.id == from {
            continue;
        }
        let center = rect.center();
        let dx = center.x - from_center.x;
        let dy = center.y - from_center.y;
        let in_direction = match dir {
            FocusDir::Left => dx < -1.0,
            FocusDir::Right => dx > 1.0,
            FocusDir::Up => dy < -1.0,
            FocusDir::Down => dy > 1.0,
        };
        if !in_direction {
            continue;
        }
        // Prefer overlap on the perpendicular axis, then proximity.
        let perp_distance = match dir {
            FocusDir::Left | FocusDir::Right => (center.y - from_center.y).abs(),
            FocusDir::Up | FocusDir::Down => (center.x - from_center.x).abs(),
        };
        let primary_distance = match dir {
            FocusDir::Left | FocusDir::Right => dx.abs(),
            FocusDir::Up | FocusDir::Down => dy.abs(),
        };
        let score = primary_distance + perp_distance * 1.5;
        match best {
            None => best = Some((score, leaf.id)),
            Some((s, _)) if score < s => best = Some((score, leaf.id)),
            _ => {}
        }
    }
    best.map(|(_, id)| id)
}

/// Cycle to the next / previous leaf in **layout order** (depth-first — same
/// order as `layout.leaves`). Used for `⌘Tab` / `⌘⇧Tab` tile focus.
pub fn focus_cycle(layout: &Layout, from: TileId, forward: bool) -> Option<TileId> {
    let n = layout.leaves.len();
    if n <= 1 {
        return None;
    }
    let idx = layout.leaves.iter().position(|(l, _)| l.id == from)?;
    let j = if forward {
        (idx + 1) % n
    } else {
        (idx + n - 1) % n
    };
    Some(layout.leaves[j].0.id)
}
