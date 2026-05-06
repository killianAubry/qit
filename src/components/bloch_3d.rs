// 3D Bloch sphere visualization with mouse-drag rotation.
//
// Single-pass software rasterizer: parametric circles (equator + two
// meridians) sampled at N points, rotated by the tile's (yaw, pitch),
// projected to 2D, then drawn segment-by-segment with depth-based alpha so
// the back of the sphere reads as occluded.
//
// Rotation state lives on the focused `LeafTile` so two bloch tiles can show
// independent angles. A drag anywhere inside a sphere updates the tile-wide
// yaw/pitch — every sphere in a tile rotates together for a consistent
// "scene" feel.
//
// In compare mode, each sphere shows a second statevector arrow (green) for
// the compare simulator.
//
// Coordinate conventions:
//   • World X → right
//   • World Y → up
//   • World Z → toward viewer
// Bloch axis (b.x, b.y, b.z) is mapped to world (b.x, b.z, b.y) so that
// the |0⟩/|1⟩ axis (Bloch Z) reads vertically.

use std::f32::consts::TAU;

use egui::{Color32, FontId, Pos2, Sense, Stroke, Vec2};

use crate::state::simulation::{bloch_from_statevector, BlochVector};
use crate::state::AppState;
use crate::theme::{color, space};
use crate::tiling::LeafTile;

const SAMPLES: usize = 64;
const MIN_ALPHA: f32 = 0.18;
const ORIGIN_EPSILON: f32 = 1e-3;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, leaf: &mut LeafTile) {
    let n_qubits = state.simulation.num_qubits;
    let has_compare = state.compare_simulation.is_some();
    header(ui, n_qubits, state, has_compare);
    ui.add_space(space::SM);

    let avail_w = ui.available_width();
    let diameter = pick_diameter(avail_w, n_qubits);

    // Always derive Bloch axes from the live statevector (partial trace), not
    // from a cached `simulation.bloch`, so the spheres cannot drift from SV.
    let bloch: Vec<BlochVector> = if state.simulation.statevector.len() == (1usize << n_qubits) {
        bloch_from_statevector(&state.simulation.statevector, n_qubits)
    } else {
        state.simulation.bloch.clone()
    };
    let n_display = bloch.len();

    // Compare Bloch vectors
    let cmp_bloch: Option<Vec<BlochVector>> = state.compare_simulation.as_ref().and_then(|cs| {
        if cs.statevector.len() == (1usize << cs.num_qubits) {
            Some(bloch_from_statevector(&cs.statevector, cs.num_qubits))
        } else {
            Some(cs.bloch.clone())
        }
    });

    egui::ScrollArea::vertical()
        .id_salt(("bloch_scroll", leaf.id.0))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Try to fit two columns side-by-side when the tile is wide
            // enough; otherwise stack vertically.
            let cols = if diameter * 2.0 + 64.0 < avail_w { 2 } else { 1 };
            let rows = (n_display + cols - 1) / cols;
            for row in 0..rows {
                ui.horizontal(|ui| {
                    for c in 0..cols {
                        let q = row * cols + c;
                        if q >= n_display {
                            break;
                        }
                        let cmp_b = cmp_bloch.as_ref().and_then(|v| v.get(q).copied());
                        draw_one(ui, leaf, q, bloch[q], cmp_b, diameter, has_compare);
                    }
                });
                ui.add_space(space::SM);
            }
        });
}

fn header(ui: &mut egui::Ui, n_qubits: usize, state: &AppState, has_compare: bool) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("bloch")
                .color(color::TEXT_MUTED)
                .monospace(),
        );
        ui.label(egui::RichText::new("·").color(color::TEXT_DIM));
        ui.label(
            egui::RichText::new(format!(
                "{} qubit{}",
                n_qubits,
                if n_qubits == 1 { "" } else { "s" },
            ))
            .color(color::TEXT_DIM)
            .monospace(),
        );
        if has_compare {
            ui.add_space(space::SM);
            ui.label(
                egui::RichText::new(state.simulator.label())
                    .color(color::ACCENT_YELLOW)
                    .monospace()
                    .size(11.0),
            );
            ui.add_space(space::XS);
            ui.label(
                egui::RichText::new("vs")
                    .color(color::TEXT_DIM)
                    .monospace()
                    .size(11.0),
            );
            ui.add_space(space::XS);
            if let Some(cmp) = state.compare_simulator {
                ui.label(
                    egui::RichText::new(cmp.label())
                        .color(color::ACCENT_GREEN)
                        .monospace()
                        .size(11.0),
                );
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new("drag to rotate")
                    .color(color::TEXT_DIM)
                    .monospace(),
            );
        });
    });
}

fn pick_diameter(avail_w: f32, n: usize) -> f32 {
    let one_col = (avail_w - 64.0).clamp(120.0, 220.0);
    if n <= 1 {
        one_col
    } else {
        // When we can fit two columns, use a smaller per-sphere diameter.
        ((avail_w - 80.0) / 2.0).clamp(110.0, 180.0)
    }
}

fn draw_one(
    ui: &mut egui::Ui,
    leaf: &mut LeafTile,
    q: usize,
    b: BlochVector,
    cmp_b: Option<BlochVector>,
    d: f32,
    _has_compare: bool,
) {
    let row_h = d + 22.0;
    let row_w = d + 96.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(row_w, row_h), Sense::click_and_drag());

    if resp.dragged() {
        let delta = resp.drag_delta();
        leaf.bloch_yaw += delta.x * 0.012;
        leaf.bloch_pitch = (leaf.bloch_pitch - delta.y * 0.012).clamp(-1.45, 1.45);
    }

    let painter = ui.painter_at(rect);
    let center = Pos2::new(rect.min.x + d * 0.5 + 8.0, rect.min.y + d * 0.5 + 6.0);
    let r = d * 0.45;

    let yaw = leaf.bloch_yaw;
    let pitch = leaf.bloch_pitch;

    // Outer silhouette — drawn first so the wireframe sits on top.
    painter.circle_stroke(center, r, Stroke::new(1.0, color::GRID_LINE));

    // Three great circles
    draw_circle(&painter, center, r, yaw, pitch, Axis::Equator, color::TEXT_MUTED);
    draw_circle(&painter, center, r, yaw, pitch, Axis::FrontMeridian, color::TEXT_DIM);
    draw_circle(&painter, center, r, yaw, pitch, Axis::SideMeridian, color::TEXT_DIM);

    // Axes
    draw_axis(&painter, center, r, yaw, pitch, [1.0, 0.0, 0.0], "x", color::ACCENT_RED);
    draw_axis(&painter, center, r, yaw, pitch, [0.0, 1.0, 0.0], "z", color::ACCENT_PURPLE);
    draw_axis(&painter, center, r, yaw, pitch, [0.0, 0.0, 1.0], "y", color::TEXT_MUTED);

    // Compare vector (drawn first, behind primary)
    if let Some(cb) = cmp_b {
        let cmp_len = (cb.x * cb.x + cb.y * cb.y + cb.z * cb.z).sqrt();
        if cmp_len > ORIGIN_EPSILON {
            let tip_world = rotate(yaw, pitch, [cb.x, cb.z, cb.y]);
            let tip_2d = project(tip_world, center, r);
            let depth_t = ((tip_world[2] + 1.0) * 0.5).clamp(0.0, 1.0);
            let tip_color = mix(color::ACCENT_GREEN, MIN_ALPHA + depth_t * (1.0 - MIN_ALPHA));
            painter.line_segment([center, tip_2d], Stroke::new(1.8, tip_color));
            painter.circle_filled(tip_2d, 3.0, tip_color);
        } else {
            // Mixed state marker for compare
            painter.circle_filled(center, 2.8, mix(color::ACCENT_GREEN, 0.8));
            painter.circle_stroke(
                center,
                5.5,
                Stroke::new(0.8, mix(color::ACCENT_GREEN, 0.22)),
            );
        }
    }

    // Primary Bloch vector (Bloch (x,y,z) → world (x, z, y)).
    let vec_len = (b.x * b.x + b.y * b.y + b.z * b.z).sqrt();
    if vec_len <= ORIGIN_EPSILON {
        // Mixed states live at the center of the Bloch ball. A zero-length
        // line looks like the vector vanished, so render an explicit marker.
        painter.circle_filled(center, 3.2, mix(color::ACCENT_YELLOW, 0.9));
        painter.circle_stroke(
            center,
            7.0,
            Stroke::new(0.9, mix(color::ACCENT_YELLOW, 0.28)),
        );
    } else {
        let tip_world = rotate(yaw, pitch, [b.x, b.z, b.y]);
        let tip_2d = project(tip_world, center, r);

        // Tail-to-head line, with depth-attenuated alpha so a vector pointing
        // away from the viewer fades behind the wireframe.
        let depth_t = ((tip_world[2] + 1.0) * 0.5).clamp(0.0, 1.0);
        let tip_color = mix(color::ACCENT_YELLOW, MIN_ALPHA + depth_t * (1.0 - MIN_ALPHA));
        painter.line_segment([center, tip_2d], Stroke::new(1.6, tip_color));
        painter.circle_filled(tip_2d, 3.5, tip_color);
    }

    // Number / label column
    let text_x = rect.min.x + d + 24.0;
    painter.text(
        Pos2::new(text_x, center.y - 22.0),
        egui::Align2::LEFT_CENTER,
        format!("q{q}"),
        FontId::monospace(13.0),
        color::TEXT_PRIMARY,
    );
    for (i, (lbl, v, c)) in [
        ("x", b.x, color::ACCENT_RED),
        ("y", b.y, color::TEXT_MUTED),
        ("z", b.z, color::ACCENT_PURPLE),
    ]
    .iter()
    .enumerate()
    {
        painter.text(
            Pos2::new(text_x, center.y - 4.0 + i as f32 * 14.0),
            egui::Align2::LEFT_CENTER,
            format!("{lbl} {v:>+.2}"),
            FontId::monospace(11.0),
            *c,
        );
    }
    painter.text(
        Pos2::new(text_x, center.y + 40.0),
        egui::Align2::LEFT_CENTER,
        if vec_len <= ORIGIN_EPSILON {
            "|r| +0.00  mixed".to_string()
        } else {
            format!("|r| {vec_len:>+.2}")
        },
        FontId::monospace(11.0),
        color::TEXT_DIM,
    );

    // Compare vector coordinates (compact, below primary)
    if let Some(cb) = cmp_b {
        let cmp_len = (cb.x * cb.x + cb.y * cb.y + cb.z * cb.z).sqrt();
        painter.text(
            Pos2::new(text_x, center.y + 54.0),
            egui::Align2::LEFT_CENTER,
            if cmp_len <= ORIGIN_EPSILON {
                "cmp  mixed".to_string()
            } else {
                format!("cmp {:.2} {:.2} {:.2}", cb.x, cb.y, cb.z)
            },
            FontId::monospace(10.0),
            mix(color::ACCENT_GREEN, 0.9),
        );
    }
}

#[derive(Clone, Copy)]
enum Axis {
    /// Bloch z = 0 (xy plane) → in world coordinates that's the xz plane.
    Equator,
    /// Bloch y = 0 plane → world xz... no, world is (bx, bz, by) so y=0 in
    /// Bloch becomes z=0 in world: the front meridian (xy plane).
    FrontMeridian,
    /// Bloch x = 0 plane → world yz: the side meridian.
    SideMeridian,
}

fn unit_circle_point(axis: Axis, t: f32) -> [f32; 3] {
    let (s, c) = t.sin_cos();
    match axis {
        // World xz plane
        Axis::Equator => [c, 0.0, s],
        // World xy plane
        Axis::FrontMeridian => [c, s, 0.0],
        // World yz plane
        Axis::SideMeridian => [0.0, c, s],
    }
}

fn draw_circle(
    painter: &egui::Painter,
    center: Pos2,
    r: f32,
    yaw: f32,
    pitch: f32,
    axis: Axis,
    base: Color32,
) {
    let mut prev_world = rotate(yaw, pitch, unit_circle_point(axis, 0.0));
    let mut prev_2d = project(prev_world, center, r);
    for i in 1..=SAMPLES {
        let t = i as f32 * TAU / SAMPLES as f32;
        let world = rotate(yaw, pitch, unit_circle_point(axis, t));
        let p2 = project(world, center, r);
        let mid_z = (prev_world[2] + world[2]) * 0.5;
        let alpha = MIN_ALPHA + ((mid_z + 1.0) * 0.5).clamp(0.0, 1.0) * (1.0 - MIN_ALPHA);
        painter.line_segment([prev_2d, p2], Stroke::new(0.7, mix(base, alpha)));
        prev_world = world;
        prev_2d = p2;
    }
}

fn draw_axis(
    painter: &egui::Painter,
    center: Pos2,
    r: f32,
    yaw: f32,
    pitch: f32,
    dir: [f32; 3],
    label: &str,
    base: Color32,
) {
    let pos = rotate(yaw, pitch, dir);
    let neg = rotate(yaw, pitch, [-dir[0], -dir[1], -dir[2]]);
    let pos2 = project(pos, center, r);
    let neg2 = project(neg, center, r);

    let alpha_pos = MIN_ALPHA + ((pos[2] + 1.0) * 0.5) * (1.0 - MIN_ALPHA);
    let alpha_neg = MIN_ALPHA + ((neg[2] + 1.0) * 0.5) * (1.0 - MIN_ALPHA);

    painter.line_segment([center, pos2], Stroke::new(0.8, mix(base, alpha_pos)));
    painter.line_segment([center, neg2], Stroke::new(0.5, mix(base, alpha_neg * 0.6)));

    painter.text(
        pos2 + Vec2::new(4.0, -2.0),
        egui::Align2::LEFT_BOTTOM,
        label,
        FontId::monospace(9.0),
        mix(base, alpha_pos),
    );
}

/// Apply yaw (around world Y) then pitch (around world X).
fn rotate(yaw: f32, pitch: f32, p: [f32; 3]) -> [f32; 3] {
    let (sy, cy) = yaw.sin_cos();
    let p1 = [cy * p[0] + sy * p[2], p[1], -sy * p[0] + cy * p[2]];
    let (sp, cp) = pitch.sin_cos();
    [p1[0], cp * p1[1] - sp * p1[2], sp * p1[1] + cp * p1[2]]
}

fn project(p: [f32; 3], center: Pos2, radius: f32) -> Pos2 {
    Pos2::new(center.x + p[0] * radius, center.y - p[1] * radius)
}

fn mix(c: Color32, alpha: f32) -> Color32 {
    let a = alpha.clamp(0.0, 1.0);
    let [r, g, b, _] = c.to_array();
    Color32::from_rgba_unmultiplied(r, g, b, (255.0 * a) as u8)
}
