use crate::camera::Camera;
use crate::sketch::{SketchPlane, SketchSession, SketchTool};
use crate::theme::Tokens;
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke};

pub struct SketchPalette;

impl SketchPalette {
    pub fn show(
        ctx: &egui::Context,
        session: &mut SketchSession,
        camera: &mut Camera,
        on_finish_extrude: &mut bool,
    ) {
        if !session.is_active {
            return;
        }

        let t = Tokens::dark();

        egui::Window::new(RichText::new("SKETCH PALETTE").strong().size(12.0))
            .id(egui::Id::new("sketch_palette_window"))
            .default_pos([960.0, 110.0])
            .default_width(220.0)
            .resizable(false)
            .collapsible(false)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(t.panel)
                    .stroke(Stroke::new(1.0_f32, t.accent))
                    .corner_radius(CornerRadius::same(6)),
            )
            .show(ctx, |ui| {
                ui.add_space(2.0);

                // Sketch Plane selector
                ui.label(RichText::new("Sketch Plane").strong().size(11.0));
                ui.horizontal(|ui| {
                    if ui.selectable_value(&mut session.plane, SketchPlane::XY, "XY (Top)").clicked() {
                        camera.set_preset_view("top");
                    }
                    if ui.selectable_value(&mut session.plane, SketchPlane::XZ, "XZ (Front)").clicked() {
                        camera.set_preset_view("front");
                    }
                    if ui.selectable_value(&mut session.plane, SketchPlane::YZ, "YZ (Right)").clicked() {
                        camera.set_preset_view("right");
                    }
                });

                ui.add_space(6.0);

                // Look At Sketch button
                if ui.button("📐 Look At Sketch").on_hover_text("Align camera to the sketch plane").clicked() {
                    match session.plane {
                        SketchPlane::XY => camera.set_preset_view("top"),
                        SketchPlane::XZ => camera.set_preset_view("front"),
                        SketchPlane::YZ => camera.set_preset_view("right"),
                    }
                }

                ui.add_space(6.0);
                ui.separator();

                // Sketch Tools
                ui.label(RichText::new("Active Tool").strong().size(11.0));
                ui.horizontal(|ui| {
                    let tools = [
                        (SketchTool::Select, "👆 Select"),
                        (SketchTool::Line, "📏 Line"),
                        (SketchTool::Rectangle, "⬛ Rect"),
                        (SketchTool::Circle, "⭕ Circle"),
                    ];
                    for (tool, label) in tools {
                        let is_sel = session.active_tool == tool;
                        let btn = egui::Button::new(label)
                            .fill(if is_sel { t.accent } else { t.panel_header });
                        if ui.add(btn).clicked() {
                            session.active_tool = tool;
                            session.pending_start = None;
                        }
                    }
                });

                ui.add_space(6.0);
                ui.separator();

                // Elements Count
                ui.label(RichText::new(format!("Elements: {}", session.elements.len())).size(11.0).color(t.text_dim));

                ui.add_space(6.0);
                ui.separator();

                // Extrude settings
                ui.label(RichText::new("Extrude Depth (mm)").strong().size(11.0));
                ui.add(egui::Slider::new(&mut session.extrude_depth, 1.0..=200.0).text("Height"));

                ui.add_space(8.0);
                ui.separator();

                // Finish / Cancel buttons
                ui.horizontal(|ui| {
                    let finish_btn = egui::Button::new(RichText::new("✓ Finish & Extrude").strong().color(Color32::WHITE))
                        .fill(Color32::from_rgb(46, 150, 90));
                    if ui.add(finish_btn).on_hover_text("Extrude sketch profiles into 3D CAD solids").clicked() {
                        *on_finish_extrude = true;
                    }

                    if ui.button("✕ Cancel").clicked() {
                        session.elements.clear();
                        session.pending_start = None;
                        session.is_active = false;
                    }
                });
            });
    }
}
