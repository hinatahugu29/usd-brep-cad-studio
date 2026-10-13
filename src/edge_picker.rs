use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use glam::{Mat4, Vec3, Vec4};

/// Decode the flat f32 edge buffer coming from the kernel ([start.xyz, end.xyz] per edge)
/// into a list of (start, end) world-space segments.
pub fn decode_edges(bytes: &[u8]) -> Vec<(Vec3, Vec3)> {
    bytes
        .chunks_exact(24)
        .map(|c| {
            let f = |i: usize| -> f32 {
                f32::from_le_bytes([c[i * 4], c[i * 4 + 1], c[i * 4 + 2], c[i * 4 + 3]])
            };
            (
                Vec3::new(f(0), f(1), f(2)),
                Vec3::new(f(3), f(4), f(5)),
            )
        })
        .collect()
}

/// Projects a 3D world position to 2D viewport coordinates. Mirrors `Gizmo::project_to_screen`.
pub fn project_to_screen(world_pos: Vec3, mvp: &Mat4, viewport_rect: Rect) -> Option<Pos2> {
    let clip = *mvp * Vec4::new(world_pos.x, world_pos.y, world_pos.z, 1.0);
    if clip.w <= 0.001 {
        return None;
    }
    let ndc_x = clip.x / clip.w;
    let ndc_y = clip.y / clip.w;

    let screen_x = viewport_rect.left() + (ndc_x * 0.5 + 0.5) * viewport_rect.width();
    let screen_y = viewport_rect.top() + (-ndc_y * 0.5 + 0.5) * viewport_rect.height();
    Some(Pos2::new(screen_x, screen_y))
}

fn dist_point_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_sq();
    if len2 < 1e-6 {
        return p.distance(a);
    }
    let t = (((p - a).dot(ab)) / len2).clamp(0.0, 1.0);
    let proj = a + ab * t;
    p.distance(proj)
}

/// Finds the nearest edge to `screen_pos`, within `threshold_px`, projecting every edge
/// with `mvp`. Returns the edge's index into `edges` (same order as the kernel's edge list).
pub fn pick_edge(
    screen_pos: Pos2,
    mvp: &Mat4,
    viewport_rect: Rect,
    edges: &[(Vec3, Vec3)],
    threshold_px: f32,
) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (i, (a, b)) in edges.iter().enumerate() {
        let (Some(sa), Some(sb)) = (
            project_to_screen(*a, mvp, viewport_rect),
            project_to_screen(*b, mvp, viewport_rect),
        ) else {
            continue;
        };
        let d = dist_point_to_segment(screen_pos, sa, sb);
        if d <= threshold_px && best.map_or(true, |(_, bd)| d < bd) {
            best = Some((i, d));
        }
    }
    best.map(|(i, _)| i)
}

/// Draws a screen-space overlay highlight for selected (gold, thick) and hovered
/// (pale gold, thin) edges on top of the 3D viewport.
pub fn draw_highlight_overlay(
    ui: &egui::Ui,
    rect: Rect,
    mvp: &Mat4,
    edges: &[(Vec3, Vec3)],
    selected: &[usize],
    hover: Option<usize>,
) {
    let painter = ui.painter_at(rect);

    for &idx in selected {
        let Some((a, b)) = edges.get(idx) else { continue };
        if let (Some(sa), Some(sb)) = (
            project_to_screen(*a, mvp, rect),
            project_to_screen(*b, mvp, rect),
        ) {
            painter.line_segment([sa, sb], Stroke::new(4.0_f32, Color32::from_rgb(255, 196, 60)));
        }
    }

    if let Some(idx) = hover {
        if !selected.contains(&idx) {
            if let Some((a, b)) = edges.get(idx) {
                if let (Some(sa), Some(sb)) = (
                    project_to_screen(*a, mvp, rect),
                    project_to_screen(*b, mvp, rect),
                ) {
                    painter.line_segment([sa, sb], Stroke::new(3.0_f32, Color32::from_rgb(255, 230, 150)));
                }
            }
        }
    }
}
