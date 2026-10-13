use crate::camera::Camera;
use eframe::egui::{self, Color32, CursorIcon, Pos2, Rect, Stroke, Vec2};
use glam::{Mat4, Vec3, Vec4};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GizmoAxis {
    None,
    X,
    Y,
    Z,
}

pub struct Gizmo {
    active_axis: GizmoAxis,
    drag_start_mouse: Pos2,
    drag_start_pos: [f64; 3],
}

impl Default for Gizmo {
    fn default() -> Self {
        Self {
            active_axis: GizmoAxis::None,
            drag_start_mouse: Pos2::ZERO,
            drag_start_pos: [0.0; 3],
        }
    }
}

impl Gizmo {
    /// Projects a 3D world position to 2D viewport coordinates
    fn project_to_screen(world_pos: Vec3, mvp: &Mat4, viewport_rect: Rect) -> Option<Pos2> {
        let clip = *mvp * Vec4::new(world_pos.x, world_pos.y, world_pos.z, 1.0);
        if clip.w <= 0.001 {
            return None;
        }
        let ndc_x = clip.x / clip.w;
        let ndc_y = clip.y / clip.w;

        // Convert NDC [-1, 1] to screen pixels
        let screen_x = viewport_rect.left() + (ndc_x * 0.5 + 0.5) * viewport_rect.width();
        let screen_y = viewport_rect.top() + (-ndc_y * 0.5 + 0.5) * viewport_rect.height();
        Some(Pos2::new(screen_x, screen_y))
    }

    /// Renders the 3D translation gizmo and handles direct viewport manipulation
    pub fn interact(
        &mut self,
        ui: &mut egui::Ui,
        camera: &Camera,
        viewport_rect: Rect,
        position: &mut [f64; 3],
        needs_rebuild: &mut bool,
    ) -> bool {
        let aspect = viewport_rect.width() / viewport_rect.height().max(1.0);
        let mvp = camera.proj_matrix(aspect) * camera.view_matrix();

        let center_world = Vec3::new(position[0] as f32, position[1] as f32, position[2] as f32);
        let Some(center_screen) = Self::project_to_screen(center_world, &mvp, viewport_rect) else {
            return false;
        };

        let axis_len_world = (camera.distance * 0.18).clamp(8.0, 80.0);

        let axes = [
            (GizmoAxis::X, Vec3::X, Color32::from_rgb(235, 75, 75), "X"),
            (GizmoAxis::Y, Vec3::Y, Color32::from_rgb(75, 215, 85), "Y"),
            (GizmoAxis::Z, Vec3::Z, Color32::from_rgb(60, 140, 255), "Z"),
        ];

        let painter = ui.painter();
        let mouse_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::ZERO);
        let mouse_down = ui.input(|i| i.pointer.primary_down());
        // `primary_clicked()` only fires on mouse-*release*, which would arm and then
        // immediately disarm `active_axis` on the same frame (mouse_down is already
        // false by then), making the gizmo impossible to drag. `primary_pressed()`
        // fires on mouse-*down*, which is what we need to start a drag.
        let mouse_pressed = ui.input(|i| i.pointer.primary_pressed());

        let mut hit_axis = GizmoAxis::None;
        let handle_radius = 12.0;

        // Draw central anchor dot
        painter.circle_filled(center_screen, 4.0, Color32::WHITE);

        for (axis, dir, color, label) in &axes {
            let tip_world = center_world + *dir * axis_len_world;
            if let Some(tip_screen) = Self::project_to_screen(tip_world, &mvp, viewport_rect) {
                let dist_to_tip = mouse_pos.distance(tip_screen);
                let is_hovered = dist_to_tip <= handle_radius;

                if is_hovered && hit_axis == GizmoAxis::None {
                    hit_axis = *axis;
                }

                let is_active = self.active_axis == *axis;
                let draw_color = if is_active || (is_hovered && self.active_axis == GizmoAxis::None) {
                    Color32::from_rgb(255, 230, 80) // Golden highlight
                } else {
                    *color
                };

                let stroke_width = if is_active || is_hovered { 3.0_f32 } else { 2.0_f32 };

                // Axis line
                painter.line_segment([center_screen, tip_screen], Stroke::new(stroke_width, draw_color));

                // Arrow head / handle circle
                painter.circle_filled(tip_screen, 7.0, draw_color);
                painter.circle_stroke(tip_screen, 7.0, Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(0, 0, 0, 160)));

                // Axis label
                painter.text(
                    tip_screen + Vec2::new(8.0, -8.0),
                    egui::Align2::LEFT_CENTER,
                    *label,
                    egui::FontId::monospace(10.0),
                    draw_color,
                );
            }
        }

        // Handle interaction state
        let mut is_manipulating = false;

        if mouse_pressed && hit_axis != GizmoAxis::None {
            self.active_axis = hit_axis;
            self.drag_start_mouse = mouse_pos;
            self.drag_start_pos = *position;
            ui.output_mut(|o| o.cursor_icon = CursorIcon::Grabbing);
        }

        if !mouse_down {
            if self.active_axis != GizmoAxis::None {
                self.active_axis = GizmoAxis::None;
                *needs_rebuild = true;
            }
        } else if self.active_axis != GizmoAxis::None {
            is_manipulating = true;
            ui.output_mut(|o| o.cursor_icon = CursorIcon::Grabbing);

            let delta_mouse = mouse_pos - self.drag_start_mouse;

            // Project screen delta onto the active axis's screen direction
            let axis_dir = match self.active_axis {
                GizmoAxis::X => Vec3::X,
                GizmoAxis::Y => Vec3::Y,
                GizmoAxis::Z => Vec3::Z,
                GizmoAxis::None => Vec3::ZERO,
            };

            let tip_world = center_world + axis_dir * axis_len_world;
            if let Some(tip_screen) = Self::project_to_screen(tip_world, &mvp, viewport_rect) {
                let screen_dir = (tip_screen - center_screen).normalized();
                let proj_delta = (delta_mouse.x * screen_dir.x + delta_mouse.y * screen_dir.y) as f64;

                let sensitivity = (camera.distance as f64 * 0.003).max(0.05);
                let world_delta = proj_delta * sensitivity;

                let axis_idx = match self.active_axis {
                    GizmoAxis::X => 0,
                    GizmoAxis::Y => 1,
                    GizmoAxis::Z => 2,
                    GizmoAxis::None => 0,
                };

                let new_val = self.drag_start_pos[axis_idx] + world_delta * 0.5;
                // Snap to 0.5mm step when shift isn't held
                let rounded = (new_val * 2.0).round() / 2.0;
                if (position[axis_idx] - rounded).abs() > 0.01 {
                    position[axis_idx] = rounded;
                    *needs_rebuild = true;
                }
            }
        }

        if hit_axis != GizmoAxis::None && !is_manipulating {
            ui.output_mut(|o| o.cursor_icon = CursorIcon::Grab);
            return true;
        }

        is_manipulating
    }
}
