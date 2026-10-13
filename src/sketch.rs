use crate::camera::Camera;
use crate::kernel::{BoolOp, ShapeDef};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, Vec2};
use glam::{Mat4, Vec3, Vec4};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SketchPlane {
    XY, // Top view (Z=0)
    XZ, // Front view (Y=0)
    YZ, // Right view (X=0)
}

impl SketchPlane {
    pub fn display_name(&self) -> &'static str {
        match self {
            SketchPlane::XY => "XY Plane (Top)",
            SketchPlane::XZ => "XZ Plane (Front)",
            SketchPlane::YZ => "YZ Plane (Right)",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SketchTool {
    Select,
    Line,
    Rectangle,
    Circle,
}

#[derive(Clone, Debug)]
pub enum SketchElement {
    Line { start: [f64; 2], end: [f64; 2] },
    Rectangle { min: [f64; 2], max: [f64; 2] },
    Circle { center: [f64; 2], radius: f64 },
}

pub struct SketchSession {
    pub is_active: bool,
    pub plane: SketchPlane,
    pub active_tool: SketchTool,
    pub elements: Vec<SketchElement>,
    pub pending_start: Option<[f64; 2]>,
    pub extrude_depth: f64,
    pub grid_step: f64,
}

impl Default for SketchSession {
    fn default() -> Self {
        Self {
            is_active: false,
            plane: SketchPlane::XY,
            active_tool: SketchTool::Rectangle,
            elements: Vec::new(),
            pending_start: None,
            extrude_depth: 20.0,
            grid_step: 5.0,
        }
    }
}

impl SketchSession {
    /// Ray-plane intersection: casts ray from camera through mouse into sketch plane
    pub fn screen_to_plane_coords(
        &self,
        mouse_pos: Pos2,
        camera: &Camera,
        viewport_rect: Rect,
    ) -> Option<[f64; 2]> {
        let aspect = viewport_rect.width() / viewport_rect.height().max(1.0);
        let view = camera.view_matrix();
        let proj = camera.proj_matrix(aspect);
        let inv_vp = (proj * view).inverse();

        // Convert screen pixel to NDC [-1, 1]
        let ndc_x = ((mouse_pos.x - viewport_rect.left()) / viewport_rect.width()) * 2.0 - 1.0;
        let ndc_y = -(((mouse_pos.y - viewport_rect.top()) / viewport_rect.height()) * 2.0 - 1.0);

        // Near and far points in clip space
        let near_clip = Vec4::new(ndc_x, ndc_y, -1.0, 1.0);
        let far_clip = Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

        let near_world_h = inv_vp * near_clip;
        let far_world_h = inv_vp * far_clip;

        if near_world_h.w.abs() < 1e-6 || far_world_h.w.abs() < 1e-6 {
            return None;
        }

        let near_world = near_world_h.truncate() / near_world_h.w;
        let far_world = far_world_h.truncate() / far_world_h.w;
        let ray_dir = (far_world - near_world).normalize_or_zero();
        let ray_orig = near_world;

        // Intersect with plane:
        match self.plane {
            SketchPlane::XY => {
                // Plane equation: z = 0
                if ray_dir.z.abs() < 1e-6 { return None; }
                let t = -ray_orig.z / ray_dir.z;
                if t < 0.0 { return None; }
                let hit = ray_orig + ray_dir * t;
                Some([hit.x as f64, hit.y as f64])
            }
            SketchPlane::XZ => {
                // Plane equation: y = 0
                if ray_dir.y.abs() < 1e-6 { return None; }
                let t = -ray_orig.y / ray_dir.y;
                if t < 0.0 { return None; }
                let hit = ray_orig + ray_dir * t;
                Some([hit.x as f64, hit.z as f64])
            }
            SketchPlane::YZ => {
                // Plane equation: x = 0
                if ray_dir.x.abs() < 1e-6 { return None; }
                let t = -ray_orig.x / ray_dir.x;
                if t < 0.0 { return None; }
                let hit = ray_orig + ray_dir * t;
                Some([hit.y as f64, hit.z as f64])
            }
        }
    }

    /// Converts 2D plane coordinate to 3D world coordinate
    pub fn plane_to_world(&self, uv: [f64; 2]) -> Vec3 {
        match self.plane {
            SketchPlane::XY => Vec3::new(uv[0] as f32, uv[1] as f32, 0.0),
            SketchPlane::XZ => Vec3::new(uv[0] as f32, 0.0, uv[1] as f32),
            SketchPlane::YZ => Vec3::new(0.0, uv[0] as f32, uv[1] as f32),
        }
    }

    /// Projects 2D plane point to screen pixels
    pub fn plane_to_screen(
        &self,
        uv: [f64; 2],
        mvp: &Mat4,
        viewport_rect: Rect,
    ) -> Option<Pos2> {
        let world = self.plane_to_world(uv);
        let clip = *mvp * Vec4::new(world.x, world.y, world.z, 1.0);
        if clip.w <= 0.001 {
            return None;
        }
        let ndc_x = clip.x / clip.w;
        let ndc_y = clip.y / clip.w;
        let sx = viewport_rect.left() + (ndc_x * 0.5 + 0.5) * viewport_rect.width();
        let sy = viewport_rect.top() + (-ndc_y * 0.5 + 0.5) * viewport_rect.height();
        Some(Pos2::new(sx, sy))
    }

    /// Renders the sketch canvas and handles interactive drawing
    pub fn handle_canvas(
        &mut self,
        ui: &mut egui::Ui,
        camera: &Camera,
        viewport_rect: Rect,
    ) {
        if !self.is_active {
            return;
        }

        let painter = ui.painter();
        let aspect = viewport_rect.width() / viewport_rect.height().max(1.0);
        let mvp = camera.proj_matrix(aspect) * camera.view_matrix();

        // 1. Draw 2D Sketch Grid on Plane
        let grid_extent = 100.0;
        let step = self.grid_step;
        let mut u = -grid_extent;
        while u <= grid_extent {
            if let (Some(p0), Some(p1)) = (
                self.plane_to_screen([u, -grid_extent], &mvp, viewport_rect),
                self.plane_to_screen([u, grid_extent], &mvp, viewport_rect),
            ) {
                let col = if u.abs() < 0.1 { Color32::from_rgb(180, 70, 70) } else { Color32::from_rgba_premultiplied(70, 80, 100, 90) };
                painter.line_segment([p0, p1], Stroke::new(1.0_f32, col));
            }
            if let (Some(p0), Some(p1)) = (
                self.plane_to_screen([-grid_extent, u], &mvp, viewport_rect),
                self.plane_to_screen([grid_extent, u], &mvp, viewport_rect),
            ) {
                let col = if u.abs() < 0.1 { Color32::from_rgb(70, 180, 70) } else { Color32::from_rgba_premultiplied(70, 80, 100, 90) };
                painter.line_segment([p0, p1], Stroke::new(1.0_f32, col));
            }
            u += step;
        }

        // 2. Draw Committed Elements
        for elem in &self.elements {
            match elem {
                SketchElement::Line { start, end } => {
                    if let (Some(s), Some(e)) = (
                        self.plane_to_screen(*start, &mvp, viewport_rect),
                        self.plane_to_screen(*end, &mvp, viewport_rect),
                    ) {
                        painter.line_segment([s, e], Stroke::new(2.5_f32, Color32::from_rgb(92, 160, 255)));
                        painter.circle_filled(s, 3.5, Color32::WHITE);
                        painter.circle_filled(e, 3.5, Color32::WHITE);
                    }
                }
                SketchElement::Rectangle { min, max } => {
                    let corners = [
                        [*min, [max[0], min[1]]],
                        [[max[0], min[1]], *max],
                        [*max, [min[0], max[1]]],
                        [[min[0], max[1]], *min],
                    ];
                    for [c0, c1] in corners {
                        if let (Some(p0), Some(p1)) = (
                            self.plane_to_screen(c0, &mvp, viewport_rect),
                            self.plane_to_screen(c1, &mvp, viewport_rect),
                        ) {
                            painter.line_segment([p0, p1], Stroke::new(2.5_f32, Color32::from_rgb(92, 160, 255)));
                            painter.circle_filled(p0, 3.5, Color32::WHITE);
                        }
                    }
                }
                SketchElement::Circle { center, radius } => {
                    let segments = 36;
                    let mut pts = Vec::new();
                    for i in 0..=segments {
                        let th = (i as f64 * std::f64::consts::TAU) / segments as f64;
                        let uv = [center[0] + radius * th.cos(), center[1] + radius * th.sin()];
                        if let Some(p) = self.plane_to_screen(uv, &mvp, viewport_rect) {
                            pts.push(p);
                        }
                    }
                    if pts.len() >= 2 {
                        for i in 0..pts.len() - 1 {
                            painter.line_segment([pts[i], pts[i + 1]], Stroke::new(2.5_f32, Color32::from_rgb(92, 160, 255)));
                        }
                    }
                }
            }
        }

        // 3. Handle Mouse Input for Drawing
        let mouse_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::ZERO);
        let clicked = ui.input(|i| i.pointer.primary_clicked());
        let current_plane_pt = self.screen_to_plane_coords(mouse_pos, camera, viewport_rect);

        if let Some(raw_pt) = current_plane_pt {
            // Snap to grid
            let snap = self.grid_step * 0.5;
            let current_uv = [
                (raw_pt[0] / snap).round() * snap,
                (raw_pt[1] / snap).round() * snap,
            ];

            // Cursor feedback
            if let Some(screen_pt) = self.plane_to_screen(current_uv, &mvp, viewport_rect) {
                painter.circle_filled(screen_pt, 4.0, Color32::from_rgb(255, 230, 80));
                let coord_str = format!("({:.1}, {:.1}) mm", current_uv[0], current_uv[1]);
                painter.text(
                    screen_pt + Vec2::new(10.0, -10.0),
                    Align2::LEFT_CENTER,
                    coord_str,
                    FontId::monospace(10.0),
                    Color32::from_rgb(255, 230, 80),
                );
            }

            // Rubber band preview while drawing
            if let Some(start_uv) = self.pending_start {
                match self.active_tool {
                    SketchTool::Line => {
                        if let (Some(s), Some(e)) = (
                            self.plane_to_screen(start_uv, &mvp, viewport_rect),
                            self.plane_to_screen(current_uv, &mvp, viewport_rect),
                        ) {
                            painter.line_segment([s, e], Stroke::new(2.0_f32, Color32::from_rgb(255, 200, 80)));
                        }
                    }
                    SketchTool::Rectangle => {
                        let corners = [
                            [start_uv, [current_uv[0], start_uv[1]]],
                            [[current_uv[0], start_uv[1]], current_uv],
                            [current_uv, [start_uv[0], current_uv[1]]],
                            [[start_uv[0], current_uv[1]], start_uv],
                        ];
                        for [c0, c1] in corners {
                            if let (Some(p0), Some(p1)) = (
                                self.plane_to_screen(c0, &mvp, viewport_rect),
                                self.plane_to_screen(c1, &mvp, viewport_rect),
                            ) {
                                painter.line_segment([p0, p1], Stroke::new(2.0_f32, Color32::from_rgb(255, 200, 80)));
                            }
                        }
                    }
                    SketchTool::Circle => {
                        let rad = ((current_uv[0] - start_uv[0]).powi(2) + (current_uv[1] - start_uv[1]).powi(2)).sqrt();
                        let segments = 32;
                        let mut pts = Vec::new();
                        for i in 0..=segments {
                            let th = (i as f64 * std::f64::consts::TAU) / segments as f64;
                            let uv = [start_uv[0] + rad * th.cos(), start_uv[1] + rad * th.sin()];
                            if let Some(p) = self.plane_to_screen(uv, &mvp, viewport_rect) {
                                pts.push(p);
                            }
                        }
                        for i in 0..pts.len().saturating_sub(1) {
                            painter.line_segment([pts[i], pts[i + 1]], Stroke::new(2.0_f32, Color32::from_rgb(255, 200, 80)));
                        }
                    }
                    _ => {}
                }

                if clicked {
                    // Commit element
                    match self.active_tool {
                        SketchTool::Line => {
                            self.elements.push(SketchElement::Line { start: start_uv, end: current_uv });
                        }
                        SketchTool::Rectangle => {
                            let min = [start_uv[0].min(current_uv[0]), start_uv[1].min(current_uv[1])];
                            let max = [start_uv[0].max(current_uv[0]), start_uv[1].max(current_uv[1])];
                            if (max[0] - min[0]).abs() > 0.1 && (max[1] - min[1]).abs() > 0.1 {
                                self.elements.push(SketchElement::Rectangle { min, max });
                            }
                        }
                        SketchTool::Circle => {
                            let radius = ((current_uv[0] - start_uv[0]).powi(2) + (current_uv[1] - start_uv[1]).powi(2)).sqrt();
                            if radius > 0.1 {
                                self.elements.push(SketchElement::Circle { center: start_uv, radius });
                            }
                        }
                        _ => {}
                    }
                    self.pending_start = None;
                }
            } else if clicked && self.active_tool != SketchTool::Select {
                self.pending_start = Some(current_uv);
            }
        }
    }

    /// Converts closed sketch elements into 3D CAD parametric shapes (Extrude)
    pub fn extrude_to_shapes(&mut self, next_id: &mut u64) -> Vec<ShapeDef> {
        let mut new_shapes = Vec::new();
        let depth = self.extrude_depth.max(1.0);

        for elem in &self.elements {
            match elem {
                SketchElement::Rectangle { min, max } => {
                    let du = (max[0] - min[0]).abs();
                    let dv = (max[1] - min[1]).abs();
                    let center_u = (min[0] + max[0]) * 0.5;
                    let center_v = (min[1] + max[1]) * 0.5;

                    let id = *next_id;
                    *next_id += 1;

                    // Assign extents directly onto their world axes (no rotation needed):
                    // the extrude direction always gets `depth`, and the sketch plane's
                    // own (u, v) axes get (du, dv) in the order they actually map to X/Y/Z.
                    let (dim_x, dim_y, dim_z, position) = match self.plane {
                        SketchPlane::XY => (du, dv, depth, [center_u, center_v, depth * 0.5]),
                        SketchPlane::XZ => (du, depth, dv, [center_u, depth * 0.5, center_v]),
                        SketchPlane::YZ => (depth, du, dv, [depth * 0.5, center_u, center_v]),
                    };

                    let mut shape = ShapeDef::new_box(id, &format!("Extrude Box {id}"), dim_x, dim_y, dim_z);
                    shape.position = position;
                    shape.op = BoolOp::Union;
                    new_shapes.push(shape);
                }
                SketchElement::Circle { center, radius } => {
                    let id = *next_id;
                    *next_id += 1;

                    let mut shape = ShapeDef::new_cylinder(id, &format!("Extrude Cyl {id}"), *radius, depth);
                    match self.plane {
                        SketchPlane::XY => {
                            shape.position = [center[0], center[1], depth * 0.5];
                        }
                        SketchPlane::XZ => {
                            shape.position = [center[0], depth * 0.5, center[1]];
                            shape.rotation = [90.0, 0.0, 0.0];
                        }
                        SketchPlane::YZ => {
                            shape.position = [depth * 0.5, center[0], center[1]];
                            shape.rotation = [0.0, 90.0, 0.0];
                        }
                    }
                    shape.op = BoolOp::Union;
                    new_shapes.push(shape);
                }
                _ => {}
            }
        }

        self.elements.clear();
        self.pending_start = None;
        self.is_active = false;
        new_shapes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A rectangle sketched on any plane should extrude with its (width, length)
    /// mapped onto that plane's own axes, and the extrude depth onto the
    /// plane's normal axis — regardless of which plane was used.
    #[test]
    fn extrude_rectangle_keeps_axes_on_all_planes() {
        for plane in [SketchPlane::XY, SketchPlane::XZ, SketchPlane::YZ] {
            let mut session = SketchSession {
                plane,
                extrude_depth: 2.0,
                ..SketchSession::default()
            };
            // u-extent (width) = 10, v-extent (length) = 4, distinct so axis swaps are detectable.
            session.elements.push(SketchElement::Rectangle {
                min: [-5.0, -2.0],
                max: [5.0, 2.0],
            });

            let mut next_id = 1u64;
            let shapes = session.extrude_to_shapes(&mut next_id);
            assert_eq!(shapes.len(), 1);
            let s = &shapes[0];

            let (expect_x, expect_y, expect_z) = match plane {
                SketchPlane::XY => (10.0, 4.0, 2.0),
                SketchPlane::XZ => (10.0, 2.0, 4.0),
                SketchPlane::YZ => (2.0, 10.0, 4.0),
            };
            assert_eq!(s.params, [expect_x, expect_y, expect_z], "wrong dims for {plane:?}");
            assert_eq!(s.rotation, [0.0, 0.0, 0.0], "no rotation should be needed for {plane:?}");
        }
    }

    /// End-to-end: extrude a rectangle on each plane and run it through the real B-rep
    /// kernel, checking the resulting solid's world-space bounding box matches what the
    /// sketch actually asked for. Confirms the fix holds all the way to the final mesh,
    /// not just the intermediate ShapeDef.
    #[test]
    fn extrude_rectangle_bbox_matches_sketch_on_xy_and_xz() {
        use crate::kernel;

        // u in [-5, 5] (width 10), v in [0, 4] (length 4), depth 2.
        let cases = [
            // (plane, expected [min_x,min_y,min_z], expected [max_x,max_y,max_z])
            (SketchPlane::XY, [-5.0, 0.0, 0.0], [5.0, 4.0, 2.0]),
            (SketchPlane::XZ, [-5.0, 0.0, 0.0], [5.0, 2.0, 4.0]),
            (SketchPlane::YZ, [0.0, -5.0, 0.0], [2.0, 5.0, 4.0]),
        ];

        for (plane, expect_min, expect_max) in cases {
            let mut session = SketchSession {
                plane,
                extrude_depth: 2.0,
                ..SketchSession::default()
            };
            session.elements.push(SketchElement::Rectangle {
                min: [-5.0, 0.0],
                max: [5.0, 4.0],
            });

            let mut next_id = 1u64;
            let shapes = session.extrude_to_shapes(&mut next_id);
            let res = kernel::evaluate_scene(&shapes, &[], None);
            assert!(res.error.is_none(), "{plane:?}: kernel error: {:?}", res.error);

            for i in 0..3 {
                assert!(
                    (res.bbox_min[i] - expect_min[i] as f32).abs() < 0.05,
                    "{plane:?}: bbox_min[{i}] = {}, expected {}",
                    res.bbox_min[i], expect_min[i]
                );
                assert!(
                    (res.bbox_max[i] - expect_max[i] as f32).abs() < 0.05,
                    "{plane:?}: bbox_max[{i}] = {}, expected {}",
                    res.bbox_max[i], expect_max[i]
                );
            }
        }
    }

    /// Simulates the full interactive pipeline: a known sketch-plane point is projected
    /// to screen pixels (same math the canvas uses to draw), then that screen position is
    /// cast back through `screen_to_plane_coords` (same math mouse clicks use). The
    /// recovered point must match the original for every plane and from several camera
    /// angles, otherwise clicks would land on the wrong spot while drawing.
    #[test]
    fn screen_roundtrip_matches_for_all_planes_and_angles() {
        let viewport_rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(1000.0, 700.0));

        let cameras = [
            Camera::default(), // default iso-ish angle
            Camera { yaw: 10.0_f32.to_radians(), pitch: 75.0_f32.to_radians(), ..Camera::default() }, // near top-down
            Camera { yaw: 200.0_f32.to_radians(), pitch: 5.0_f32.to_radians(), ..Camera::default() }, // near-level, looking from behind
        ];

        let sample_uvs: [[f64; 2]; 4] = [[0.0, 0.0], [12.5, -7.0], [-20.0, 30.0], [5.5, 5.5]];

        for plane in [SketchPlane::XY, SketchPlane::XZ, SketchPlane::YZ] {
            let session = SketchSession { plane, ..SketchSession::default() };
            for camera in &cameras {
                let aspect = viewport_rect.width() / viewport_rect.height();
                let mvp = camera.proj_matrix(aspect) * camera.view_matrix();

                for uv in sample_uvs {
                    let Some(screen_pt) = session.plane_to_screen(uv, &mvp, viewport_rect) else {
                        continue; // behind camera for this angle; skip
                    };
                    // Only meaningful if the projected point actually lands in the viewport.
                    if !viewport_rect.contains(screen_pt) {
                        continue;
                    }
                    let Some(recovered) = session.screen_to_plane_coords(screen_pt, camera, viewport_rect) else {
                        panic!("round-trip failed to hit {plane:?} plane for uv={uv:?}");
                    };
                    assert!(
                        (recovered[0] - uv[0]).abs() < 0.01 && (recovered[1] - uv[1]).abs() < 0.01,
                        "{plane:?}: round-trip mismatch, sent {uv:?} got {recovered:?}"
                    );
                }
            }
        }
    }
}
