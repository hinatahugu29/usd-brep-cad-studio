use crate::camera::Camera;
use crate::kernel::{EvaluationResult, ShapeDef};
use crate::theme::Tokens;
use eframe::egui::{self, RichText};

pub struct Browser {
    pub show_origin: bool,
    pub show_axes: bool,
    pub show_planes: bool,
    pub material_name: &'static str,
    pub material_density: f64, // g/cm^3
}

impl Default for Browser {
    fn default() -> Self {
        Self {
            show_origin: false,
            show_axes: true,
            show_planes: false,
            material_name: "Steel (Mild)",
            material_density: 7.85,
        }
    }
}

impl Browser {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        shapes: &mut Vec<ShapeDef>,
        selected_id: &mut Option<u64>,
        eval_result: &EvaluationResult,
        camera: &mut Camera,
        needs_rebuild: &mut bool,
    ) {
        let t = Tokens::dark();

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("BROWSER").strong().size(13.0).color(t.text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("Part 1").color(t.accent).size(11.0));
            });
        });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // 1. Document Settings
            egui::CollapsingHeader::new(RichText::new("⚙ Document Settings").size(12.0))
                .default_open(false)
                .show(ui, |ui| {
                    ui.indent("doc_settings_indent", |ui| {
                        ui.label(RichText::new("• Standard Units: mm").size(11.0).color(t.text_dim));
                        ui.label(RichText::new("• Precision: 0.001 mm").size(11.0).color(t.text_dim));
                        ui.label(RichText::new("• Kernel: USD-BRep SMLib").size(11.0).color(t.text_dim));
                    });
                });

            // 2. Named Views
            egui::CollapsingHeader::new(RichText::new("📷 Named Views").size(12.0))
                .default_open(false)
                .show(ui, |ui| {
                    ui.indent("named_views_indent", |ui| {
                        if ui.selectable_label(false, "⌂ Isometric").clicked() {
                            camera.set_preset_view("iso");
                        }
                        if ui.selectable_label(false, "🔲 Top").clicked() {
                            camera.set_preset_view("top");
                        }
                        if ui.selectable_label(false, "🔲 Front").clicked() {
                            camera.set_preset_view("front");
                        }
                        if ui.selectable_label(false, "🔲 Right").clicked() {
                            camera.set_preset_view("right");
                        }
                    });
                });

            // 3. Origin
            egui::CollapsingHeader::new(RichText::new("🎯 Origin").size(12.0))
                .default_open(true)
                .show(ui, |ui| {
                    ui.indent("origin_indent", |ui| {
                        ui.horizontal(|ui| {
                            let icon = if self.show_origin { "👁" } else { "🚫" };
                            if ui.small_button(icon).clicked() {
                                self.show_origin = !self.show_origin;
                            }
                            ui.label("Origin Point (0,0,0)");
                        });
                        ui.horizontal(|ui| {
                            let icon = if self.show_axes { "👁" } else { "🚫" };
                            if ui.small_button(icon).clicked() {
                                self.show_axes = !self.show_axes;
                            }
                            ui.label("Axes (X, Y, Z)");
                        });
                        ui.horizontal(|ui| {
                            let icon = if self.show_planes { "👁" } else { "🚫" };
                            if ui.small_button(icon).clicked() {
                                self.show_planes = !self.show_planes;
                            }
                            ui.label("Planes (XY, XZ, YZ)");
                        });
                    });
                });

            // 4. Bodies
            egui::CollapsingHeader::new(RichText::new("🧊 Bodies (1)").size(12.0))
                .default_open(true)
                .show(ui, |ui| {
                    ui.indent("bodies_indent", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("👁 Solid Body 1");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(RichText::new(format!("{} tris", eval_result.triangle_count)).size(10.0).color(t.text_dim));
                            });
                        });
                        ui.label(RichText::new("  Material: Steel (Metallic Blue)").size(10.5).color(t.text_dim));
                    });
                });

            // 5. Features Tree
            egui::CollapsingHeader::new(RichText::new("⏳ Features Tree").size(12.0))
                .default_open(true)
                .show(ui, |ui| {
                    ui.indent("features_indent", |ui| {
                        let mut to_remove = None;
                        for shape in shapes.iter_mut() {
                            let is_selected = *selected_id == Some(shape.id);
                            ui.horizontal(|ui| {
                                let vis_icon = if shape.visible { "👁" } else { "🚫" };
                                if ui.small_button(vis_icon).clicked() {
                                    shape.visible = !shape.visible;
                                    *needs_rebuild = true;
                                }

                                let label_text = format!("{} {}", shape.kind.display_name(), shape.name);
                                if ui.selectable_label(is_selected, label_text).clicked() {
                                    *selected_id = Some(shape.id);
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.small_button("🗑").clicked() {
                                        to_remove = Some(shape.id);
                                    }
                                });
                            });
                        }

                        if let Some(id) = to_remove {
                            shapes.retain(|s| s.id != id);
                            if *selected_id == Some(id) {
                                *selected_id = shapes.first().map(|s| s.id);
                            }
                            *needs_rebuild = true;
                        }
                    });
                });

            ui.add_space(12.0);
            ui.separator();

            // 6. Analytics Section
            egui::CollapsingHeader::new(RichText::new("📊 Mass & Physical Properties").size(12.0))
                .default_open(true)
                .show(ui, |ui| {
                    ui.indent("analytics_indent", |ui| {
                        // Material selector
                        ui.horizontal(|ui| {
                            ui.label("Material:");
                            egui::ComboBox::from_id_salt("mat_density_combo")
                                .selected_text(self.material_name)
                                .show_ui(ui, |ui| {
                                    if ui.selectable_value(&mut self.material_density, 7.85, "Steel (7.85 g/cm³)").clicked() {
                                        self.material_name = "Steel (Mild)";
                                    }
                                    if ui.selectable_value(&mut self.material_density, 2.70, "Aluminum (2.70 g/cm³)").clicked() {
                                        self.material_name = "Aluminum (6061)";
                                    }
                                    if ui.selectable_value(&mut self.material_density, 8.50, "Brass (8.50 g/cm³)").clicked() {
                                        self.material_name = "Brass";
                                    }
                                    if ui.selectable_value(&mut self.material_density, 4.51, "Titanium (4.51 g/cm³)").clicked() {
                                        self.material_name = "Titanium";
                                    }
                                    if ui.selectable_value(&mut self.material_density, 1.04, "ABS Plastic (1.04 g/cm³)").clicked() {
                                        self.material_name = "ABS Plastic";
                                    }
                                });
                        });

                        let mass_g = (eval_result.volume * self.material_density) / 1000.0;
                        ui.label(RichText::new(format!("• Mass: {:.2} g ({:.4} kg)", mass_g, mass_g / 1000.0)).strong().color(t.accent));
                        ui.label(format!("• Volume: {:.2} mm³", eval_result.volume));
                        ui.label(format!("• Surface Area: {:.2} mm²", eval_result.surface_area));
                        ui.label(format!("• Density: {:.2} g/cm³", self.material_density));
                        ui.label(format!("• Triangles: {}", eval_result.triangle_count));
                        ui.add_space(4.0);
                        ui.label(RichText::new(format!(
                            "BBox:\n[{:.1}, {:.1}, {:.1}] to\n[{:.1}, {:.1}, {:.1}]",
                            eval_result.bbox_min[0], eval_result.bbox_min[1], eval_result.bbox_min[2],
                            eval_result.bbox_max[0], eval_result.bbox_max[1], eval_result.bbox_max[2]
                        )).size(10.5).color(t.text_dim));
                    });
                });
        });
    }
}
