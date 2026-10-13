use crate::kernel::ShapeDef;
use crate::theme::Tokens;
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke};

#[derive(Clone, Debug)]
pub struct ParamEntry {
    pub name: String,
    pub value: f64,
    pub unit: &'static str,
    pub comment: String,
}

pub struct UserParams {
    pub is_open: bool,
    pub params: Vec<ParamEntry>,
    pub new_name: String,
    pub new_val: f64,
}

impl Default for UserParams {
    fn default() -> Self {
        Self {
            is_open: false,
            params: vec![
                ParamEntry {
                    name: "base_width".to_string(),
                    value: 30.0,
                    unit: "mm",
                    comment: "Width of base box".to_string(),
                },
                ParamEntry {
                    name: "base_length".to_string(),
                    value: 30.0,
                    unit: "mm",
                    comment: "Length of base box".to_string(),
                },
                ParamEntry {
                    name: "base_height".to_string(),
                    value: 15.0,
                    unit: "mm",
                    comment: "Height of base box".to_string(),
                },
                ParamEntry {
                    name: "hole_radius".to_string(),
                    value: 8.0,
                    unit: "mm",
                    comment: "Radius of center hole".to_string(),
                },
            ],
            new_name: "thickness".to_string(),
            new_val: 5.0,
        }
    }
}

impl UserParams {
    /// Applies named parameters to corresponding starter shapes if names match
    pub fn sync_to_shapes(&self, shapes: &mut [ShapeDef], needs_rebuild: &mut bool) {
        for p in &self.params {
            match p.name.as_str() {
                "base_width" => {
                    if let Some(s) = shapes.iter_mut().find(|s| s.id == 1) {
                        if (s.params[0] - p.value).abs() > 0.001 {
                            s.params[0] = p.value;
                            *needs_rebuild = true;
                        }
                    }
                }
                "base_length" => {
                    if let Some(s) = shapes.iter_mut().find(|s| s.id == 1) {
                        if (s.params[1] - p.value).abs() > 0.001 {
                            s.params[1] = p.value;
                            *needs_rebuild = true;
                        }
                    }
                }
                "base_height" => {
                    if let Some(s) = shapes.iter_mut().find(|s| s.id == 1) {
                        if (s.params[2] - p.value).abs() > 0.001 {
                            s.params[2] = p.value;
                            *needs_rebuild = true;
                        }
                    }
                }
                "hole_radius" => {
                    if let Some(s) = shapes.iter_mut().find(|s| s.id == 2) {
                        if (s.params[0] - p.value).abs() > 0.001 {
                            s.params[0] = p.value;
                            *needs_rebuild = true;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn show_dialog(
        &mut self,
        ctx: &egui::Context,
        shapes: &mut [ShapeDef],
        needs_rebuild: &mut bool,
    ) {
        if !self.is_open {
            return;
        }

        let t = Tokens::dark();

        egui::Window::new(RichText::new("⚙ Parameters (Parametric Variables)").strong())
            .id(egui::Id::new("user_parameters_dialog"))
            .default_pos([400.0, 180.0])
            .default_width(420.0)
            .resizable(true)
            .collapsible(true)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(t.panel)
                    .stroke(Stroke::new(1.0_f32, t.accent))
                    .corner_radius(CornerRadius::same(6)),
            )
            .show(ctx, |ui| {
                ui.label(RichText::new("Change a parameter and watch the USD-BRep model rebuild automatically.").size(11.0).color(t.text_dim));
                ui.add_space(4.0);
                ui.separator();

                let mut to_remove = None;
                let mut changed = false;

                egui::Grid::new("params_grid")
                    .striped(true)
                    .min_col_width(70.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new("Name").strong().color(Color32::WHITE));
                        ui.label(RichText::new("Unit").strong().color(t.text_dim));
                        ui.label(RichText::new("Value").strong().color(Color32::WHITE));
                        ui.label(RichText::new("Comment").strong().color(Color32::WHITE));
                        ui.label("");
                        ui.end_row();

                        for (idx, p) in self.params.iter_mut().enumerate() {
                            ui.label(RichText::new(&p.name).monospace().color(t.accent));
                            ui.label(RichText::new(p.unit).size(10.5).color(t.text_dim));
                            changed |= ui.add(egui::DragValue::new(&mut p.value).speed(0.5).range(0.1..=500.0)).changed();
                            ui.label(RichText::new(&p.comment).size(10.5).color(t.text_dim));
                            if ui.small_button("🗑").clicked() {
                                to_remove = Some(idx);
                            }
                            ui.end_row();
                        }
                    });

                if let Some(idx) = to_remove {
                    self.params.remove(idx);
                }

                if changed {
                    self.sync_to_shapes(shapes, needs_rebuild);
                }

                ui.add_space(8.0);
                ui.separator();

                // Add new parameter row
                ui.horizontal(|ui| {
                    ui.label("Add:");
                    ui.text_edit_singleline(&mut self.new_name);
                    ui.add(egui::DragValue::new(&mut self.new_val).speed(0.5));
                    if ui.button("➕ Add").clicked() && !self.new_name.is_empty() {
                        self.params.push(ParamEntry {
                            name: self.new_name.clone(),
                            value: self.new_val,
                            unit: "mm",
                            comment: "User custom variable".to_string(),
                        });
                        self.new_name = format!("param_{}", self.params.len() + 1);
                    }
                });

                ui.add_space(4.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        self.is_open = false;
                    }
                });
            });
    }
}
