use crate::camera::Camera;
use crate::kernel::ShapeDef;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, Vec2};
use glam::{Mat4, Vec3, Vec4};

pub struct MeasureTool {
    pub show_dimensions: bool,
}

impl Default for MeasureTool {
    fn default() -> Self {
        Self {
            show_dimensions: false,
        }
    }
}

impl MeasureTool {
    fn project_to_screen(world_pos: Vec3, mvp: &Mat4, viewport_rect: Rect) -> Option<Pos2> {
        let clip = *mvp * Vec4::new(world_pos.x, world_pos.y, world_pos.z, 1.0);
        if clip.w <= 0.001 {
            return None;
        }
        let ndc_x = clip.x / clip.w;
        let ndc_y = clip.y / clip.w;
        let sx = viewport_rect.left() + (ndc_x * 0.5 + 0.5) * viewport_rect.width();
        let sy = viewport_rect.top() + (-ndc_y * 0.5 + 0.5) * viewport_rect.height();
        Some(Pos2::new(sx, sy))
    }

    /// Renders technical CAD dimension extension lines and badges over selected or all solids
    pub fn render_dimensions(
        &self,
        ui: &mut egui::Ui,
        camera: &Camera,
        viewport_rect: Rect,
        shapes: &[ShapeDef],
        selected_id: Option<u64>,
    ) {
        if !self.show_dimensions {
            return;
        }

        let aspect = viewport_rect.width() / viewport_rect.height().max(1.0);
        let mvp = camera.proj_matrix(aspect) * camera.view_matrix();
        let painter = ui.painter();

        let target_shapes: Vec<&ShapeDef> = if let Some(sel_id) = selected_id {
            shapes.iter().filter(|s| s.id == sel_id).collect()
        } else {
            shapes.iter().collect()
        };

        let dim_stroke = Stroke::new(1.2_f32, Color32::from_rgb(100, 200, 255));
        let guide_stroke = Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(140, 160, 190, 120));

        for shape in target_shapes {
            let (dim_x, dim_y, dim_z) = match shape.kind {
                crate::kernel::PrimitiveKind::Box => (shape.params[0], shape.params[1], shape.params[2]),
                crate::kernel::PrimitiveKind::Cylinder => (shape.params[0] * 2.0, shape.params[0] * 2.0, shape.params[1]),
                crate::kernel::PrimitiveKind::Sphere => (shape.params[0] * 2.0, shape.params[0] * 2.0, shape.params[0] * 2.0),
                crate::kernel::PrimitiveKind::Cone => (shape.params[0] * 2.0, shape.params[0] * 2.0, shape.params[2]),
                crate::kernel::PrimitiveKind::Torus => ((shape.params[0] + shape.params[1]) * 2.0, (shape.params[0] + shape.params[1]) * 2.0, shape.params[1] * 2.0),
            };

            let hx = (dim_x * 0.5) as f32;
            let hy = (dim_y * 0.5) as f32;
            let hz = (dim_z * 0.5) as f32;

            let center = Vec3::new(shape.position[0] as f32, shape.position[1] as f32, shape.position[2] as f32);

            // Bounding box corners
            let p_min = center + Vec3::new(-hx, -hy, -hz);
            let p_x = center + Vec3::new(hx, -hy, -hz);
            let p_y = center + Vec3::new(-hx, hy, -hz);
            let p_z = center + Vec3::new(-hx, -hy, hz);

            // 1. Dimension X (Width)
            if let (Some(s0), Some(s1)) = (
                Self::project_to_screen(p_min, &mvp, viewport_rect),
                Self::project_to_screen(p_x, &mvp, viewport_rect),
            ) {
                let offset = Vec2::new(0.0, 16.0);
                let d0 = s0 + offset;
                let d1 = s1 + offset;

                painter.line_segment([s0, d0], guide_stroke);
                painter.line_segment([s1, d1], guide_stroke);
                painter.line_segment([d0, d1], dim_stroke);

                let mid = Pos2::new((d0.x + d1.x) * 0.5, (d0.y + d1.y) * 0.5);
                let badge_rect = Rect::from_center_size(mid, Vec2::new(56.0, 16.0));
                painter.rect_filled(badge_rect, 3.0, Color32::from_rgba_premultiplied(20, 24, 32, 230));
                painter.rect_stroke(badge_rect, 3.0, Stroke::new(1.0_f32, Color32::from_rgb(100, 200, 255)), egui::StrokeKind::Inside);
                painter.text(
                    mid,
                    Align2::CENTER_CENTER,
                    format!("{:.1} mm", dim_x),
                    FontId::monospace(10.0),
                    Color32::from_rgb(130, 220, 255),
                );
            }

            // 2. Dimension Y (Length)
            if let (Some(s0), Some(s1)) = (
                Self::project_to_screen(p_min, &mvp, viewport_rect),
                Self::project_to_screen(p_y, &mvp, viewport_rect),
            ) {
                let offset = Vec2::new(-16.0, 0.0);
                let d0 = s0 + offset;
                let d1 = s1 + offset;

                painter.line_segment([s0, d0], guide_stroke);
                painter.line_segment([s1, d1], guide_stroke);
                painter.line_segment([d0, d1], dim_stroke);

                let mid = Pos2::new((d0.x + d1.x) * 0.5, (d0.y + d1.y) * 0.5);
                let badge_rect = Rect::from_center_size(mid, Vec2::new(56.0, 16.0));
                painter.rect_filled(badge_rect, 3.0, Color32::from_rgba_premultiplied(20, 24, 32, 230));
                painter.rect_stroke(badge_rect, 3.0, Stroke::new(1.0_f32, Color32::from_rgb(100, 200, 255)), egui::StrokeKind::Inside);
                painter.text(
                    mid,
                    Align2::CENTER_CENTER,
                    format!("{:.1} mm", dim_y),
                    FontId::monospace(10.0),
                    Color32::from_rgb(130, 220, 255),
                );
            }

            // 3. Dimension Z (Height)
            if let (Some(s0), Some(s1)) = (
                Self::project_to_screen(p_min, &mvp, viewport_rect),
                Self::project_to_screen(p_z, &mvp, viewport_rect),
            ) {
                let offset = Vec2::new(16.0, 0.0);
                let d0 = s0 + offset;
                let d1 = s1 + offset;

                painter.line_segment([s0, d0], guide_stroke);
                painter.line_segment([s1, d1], guide_stroke);
                painter.line_segment([d0, d1], dim_stroke);

                let mid = Pos2::new((d0.x + d1.x) * 0.5, (d0.y + d1.y) * 0.5);
                let badge_rect = Rect::from_center_size(mid, Vec2::new(56.0, 16.0));
                painter.rect_filled(badge_rect, 3.0, Color32::from_rgba_premultiplied(20, 24, 32, 230));
                painter.rect_stroke(badge_rect, 3.0, Stroke::new(1.0_f32, Color32::from_rgb(100, 200, 255)), egui::StrokeKind::Inside);
                painter.text(
                    mid,
                    Align2::CENTER_CENTER,
                    format!("{:.1} mm", dim_z),
                    FontId::monospace(10.0),
                    Color32::from_rgb(130, 220, 255),
                );
            }
        }
    }
}
