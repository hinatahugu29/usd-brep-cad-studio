use crate::kernel::{BoolOp, PrimitiveKind, ShapeDef};
use crate::theme::Tokens;
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke};

pub struct FeatureDialog;

impl FeatureDialog {
    pub fn show(
        ctx: &egui::Context,
        shapes: &mut [ShapeDef],
        selected_id: Option<u64>,
        needs_rebuild: &mut bool,
    ) {
        let t = Tokens::dark();
        let Some(sel_id) = selected_id else { return };
        let Some(shape) = shapes.iter_mut().find(|s| s.id == sel_id) else { return };

        let window_title = format!("{} Parameters", shape.kind.display_name());

        egui::Window::new(RichText::new(&window_title).strong())
            .id(egui::Id::new("feature_param_dialog"))
            .default_pos([340.0, 110.0])
            .default_width(280.0)
            .resizable(true)
            .collapsible(true)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(t.panel)
                    .stroke(Stroke::new(1.0_f32, t.border))
                    .corner_radius(CornerRadius::same(6)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut shape.name);
                });

                ui.add_space(4.0);
                ui.separator();

                // Boolean Operation selection chips
                ui.label(RichText::new("Operation").strong().size(11.0));
                ui.horizontal(|ui| {
                    let ops = [
                        (BoolOp::Base, "Base"),
                        (BoolOp::Union, "+ Join"),
                        (BoolOp::Cut, "- Cut"),
                        (BoolOp::Intersect, "∩ Intersect"),
                    ];
                    for (op, label) in ops {
                        let is_active = shape.op == op;
                        let btn = egui::Button::new(label)
                            .fill(if is_active { t.accent } else { t.panel_header });
                        if ui.add(btn).clicked() {
                            shape.op = op;
                            *needs_rebuild = true;
                        }
                    }
                });

                ui.add_space(6.0);
                ui.separator();

                // Dimensions
                ui.label(RichText::new("Dimensions (mm)").strong().size(11.0));
                match shape.kind {
                    PrimitiveKind::Box => {
                        let mut changed = false;
                        changed |= ui.add(egui::Slider::new(&mut shape.params[0], 0.1..=200.0).text("Width (X)")).changed();
                        changed |= ui.add(egui::Slider::new(&mut shape.params[1], 0.1..=200.0).text("Length (Y)")).changed();
                        changed |= ui.add(egui::Slider::new(&mut shape.params[2], 0.1..=200.0).text("Height (Z)")).changed();
                        if changed { *needs_rebuild = true; }
                    }
                    PrimitiveKind::Cylinder => {
                        let mut changed = false;
                        changed |= ui.add(egui::Slider::new(&mut shape.params[0], 0.1..=100.0).text("Radius")).changed();
                        changed |= ui.add(egui::Slider::new(&mut shape.params[1], 0.1..=200.0).text("Height")).changed();
                        if changed { *needs_rebuild = true; }
                    }
                    PrimitiveKind::Sphere => {
                        if ui.add(egui::Slider::new(&mut shape.params[0], 0.1..=100.0).text("Radius")).changed() {
                            *needs_rebuild = true;
                        }
                    }
                    PrimitiveKind::Cone => {
                        let mut changed = false;
                        changed |= ui.add(egui::Slider::new(&mut shape.params[0], 0.0..=100.0).text("Base Radius")).changed();
                        changed |= ui.add(egui::Slider::new(&mut shape.params[1], 0.0..=100.0).text("Top Radius")).changed();
                        changed |= ui.add(egui::Slider::new(&mut shape.params[2], 0.1..=200.0).text("Height")).changed();
                        if changed { *needs_rebuild = true; }
                    }
                    PrimitiveKind::Torus => {
                        let mut changed = false;
                        changed |= ui.add(egui::Slider::new(&mut shape.params[0], 1.0..=100.0).text("Major Radius")).changed();
                        changed |= ui.add(egui::Slider::new(&mut shape.params[1], 0.1..=30.0).text("Minor Radius")).changed();
                        if changed { *needs_rebuild = true; }
                    }
                }

                ui.add_space(6.0);
                ui.separator();

                // Position Offset
                ui.label(RichText::new("Position Offset (mm)").strong().size(11.0));
                let mut pos_changed = false;
                pos_changed |= ui.add(egui::Slider::new(&mut shape.position[0], -100.0..=100.0).text("X")).changed();
                pos_changed |= ui.add(egui::Slider::new(&mut shape.position[1], -100.0..=100.0).text("Y")).changed();
                pos_changed |= ui.add(egui::Slider::new(&mut shape.position[2], -100.0..=100.0).text("Z")).changed();
                if pos_changed { *needs_rebuild = true; }

                ui.add_space(6.0);
                ui.separator();

                // Rotation
                ui.label(RichText::new("Rotation (deg)").strong().size(11.0));
                let mut rot_changed = false;
                rot_changed |= ui.add(egui::Slider::new(&mut shape.rotation[0], -180.0..=180.0).text("Rot X")).changed();
                rot_changed |= ui.add(egui::Slider::new(&mut shape.rotation[1], -180.0..=180.0).text("Rot Y")).changed();
                rot_changed |= ui.add(egui::Slider::new(&mut shape.rotation[2], -180.0..=180.0).text("Rot Z")).changed();
                if rot_changed { *needs_rebuild = true; }

                ui.add_space(8.0);
                ui.separator();

                // Material & Appearance
                ui.label(RichText::new("Material Appearance").strong().size(11.0));
                let materials = [
                    ([0.52_f32, 0.65, 0.82], "Steel", Color32::from_rgb(130, 165, 210)),
                    ([0.78_f32, 0.82, 0.86], "Silver", Color32::from_rgb(200, 210, 220)),
                    ([0.85_f32, 0.72, 0.35], "Brass", Color32::from_rgb(220, 185, 90)),
                    ([0.85_f32, 0.30, 0.30], "Crimson", Color32::from_rgb(220, 80, 80)),
                    ([0.35_f32, 0.75, 0.45], "Green", Color32::from_rgb(90, 190, 115)),
                    ([0.22_f32, 0.24, 0.28], "Carbon", Color32::from_rgb(55, 60, 70)),
                ];

                ui.horizontal_wrapped(|ui| {
                    for (rgb, name, col32) in materials {
                        let is_active = (shape.color[0] - rgb[0]).abs() < 0.05 && (shape.color[1] - rgb[1]).abs() < 0.05;
                        let btn = egui::Button::new(RichText::new(name).size(10.5).color(Color32::WHITE))
                            .fill(if is_active { t.accent } else { col32 });
                        if ui.add(btn).clicked() {
                            shape.color = rgb;
                            *needs_rebuild = true;
                        }
                    }
                });
            });
    }
}
