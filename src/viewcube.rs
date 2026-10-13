use crate::camera::Camera;
use crate::theme::Tokens;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

pub struct ViewCube;

impl ViewCube {
    pub fn show(ui: &mut egui::Ui, camera: &mut Camera, viewport_rect: Rect) {
        let t = Tokens::dark();
        let cube_size = 90.0;
        let margin = 16.0;

        let cube_rect = Rect::from_min_size(
            Pos2::new(
                viewport_rect.right() - cube_size - margin,
                viewport_rect.top() + margin,
            ),
            Vec2::splat(cube_size),
        );

        let painter = ui.painter();

        // Background chip
        painter.rect(
            cube_rect,
            8.0,
            Color32::from_rgba_premultiplied(33, 37, 45, 210),
            Stroke::new(1.0_f32, t.border),
            egui::StrokeKind::Inside,
        );

        // Header / Compass indicator
        let center = cube_rect.center();

        // Interactive Face Buttons: Top, Front, Right, Iso in a CAD layout
        let btn_size = Vec2::new(36.0, 20.0);

        // Top button
        let top_r = Rect::from_center_size(center + Vec2::new(0.0, -22.0), btn_size);
        let top_resp = ui.interact(top_r, ui.id().with("vc_top"), Sense::click());
        let top_bg = if top_resp.hovered() { t.accent } else { t.panel_header };
        painter.rect_filled(top_r, 4.0, top_bg);
        painter.text(top_r.center(), Align2::CENTER_CENTER, "TOP", FontId::proportional(10.0), Color32::WHITE);
        if top_resp.clicked() {
            camera.set_preset_view("top");
        }

        // Front button
        let front_r = Rect::from_center_size(center + Vec2::new(-20.0, 4.0), btn_size);
        let front_resp = ui.interact(front_r, ui.id().with("vc_front"), Sense::click());
        let front_bg = if front_resp.hovered() { t.accent } else { t.panel_header };
        painter.rect_filled(front_r, 4.0, front_bg);
        painter.text(front_r.center(), Align2::CENTER_CENTER, "FRONT", FontId::proportional(10.0), Color32::WHITE);
        if front_resp.clicked() {
            camera.set_preset_view("front");
        }

        // Right button
        let right_r = Rect::from_center_size(center + Vec2::new(20.0, 4.0), btn_size);
        let right_resp = ui.interact(right_r, ui.id().with("vc_right"), Sense::click());
        let right_bg = if right_resp.hovered() { t.accent } else { t.panel_header };
        painter.rect_filled(right_r, 4.0, right_bg);
        painter.text(right_r.center(), Align2::CENTER_CENTER, "RIGHT", FontId::proportional(10.0), Color32::WHITE);
        if right_resp.clicked() {
            camera.set_preset_view("right");
        }

        // Iso / Home button
        let iso_r = Rect::from_center_size(center + Vec2::new(0.0, 27.0), Vec2::new(48.0, 18.0));
        let iso_resp = ui.interact(iso_r, ui.id().with("vc_iso"), Sense::click());
        let iso_bg = if iso_resp.hovered() { t.accent } else { Color32::from_rgb(45, 52, 65) };
        painter.rect_filled(iso_r, 4.0, iso_bg);
        painter.text(iso_r.center(), Align2::CENTER_CENTER, "⌂ ISO", FontId::proportional(10.0), Color32::from_rgb(200, 220, 250));
        if iso_resp.clicked() {
            camera.set_preset_view("iso");
        }

        // Coordinate axes triad at bottom-left of viewport
        let axis_origin = Pos2::new(viewport_rect.left() + 45.0, viewport_rect.bottom() - 45.0);
        let axis_len = 28.0;

        // Project X, Y, Z axes using camera orientation
        let fwd = (camera.target - camera.eye_position()).normalize_or_zero();
        let right = fwd.cross(glam::Vec3::Y).normalize_or_zero();
        let up = right.cross(fwd).normalize_or_zero();

        let project_axis = |world_dir: glam::Vec3| -> Vec2 {
            let x = world_dir.dot(right);
            let y = -world_dir.dot(up);
            Vec2::new(x * axis_len, y * axis_len)
        };

        let x_2d = project_axis(glam::Vec3::X);
        let y_2d = project_axis(glam::Vec3::Y);
        let z_2d = project_axis(glam::Vec3::Z);

        // Draw axis lines and labels
        painter.line_segment([axis_origin, axis_origin + x_2d], Stroke::new(2.5_f32, Color32::from_rgb(235, 75, 75)));
        painter.text(axis_origin + x_2d + Vec2::new(4.0, -2.0), Align2::LEFT_CENTER, "X", FontId::monospace(11.0), Color32::from_rgb(255, 120, 120));

        painter.line_segment([axis_origin, axis_origin + y_2d], Stroke::new(2.5_f32, Color32::from_rgb(75, 210, 85)));
        painter.text(axis_origin + y_2d + Vec2::new(4.0, -2.0), Align2::LEFT_CENTER, "Y", FontId::monospace(11.0), Color32::from_rgb(120, 240, 130));

        painter.line_segment([axis_origin, axis_origin + z_2d], Stroke::new(2.5_f32, Color32::from_rgb(60, 140, 245)));
        painter.text(axis_origin + z_2d + Vec2::new(4.0, -2.0), Align2::LEFT_CENTER, "Z", FontId::monospace(11.0), Color32::from_rgb(130, 185, 255));
    }
}
