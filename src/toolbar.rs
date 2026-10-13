use crate::kernel::{EdgeTreatment, PrimitiveKind};
use crate::theme::Tokens;
use eframe::egui::{self, Color32, RichText, Stroke};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WorkspaceTab {
    Solid,
    Surface,
    Mesh,
    SheetMetal,
    Utilities,
}

pub struct Toolbar {
    pub current_tab: WorkspaceTab,
}

impl Default for Toolbar {
    fn default() -> Self {
        Self {
            current_tab: WorkspaceTab::Solid,
        }
    }
}

pub enum ToolbarAction {
    None,
    AddShape(PrimitiveKind),
    SetEdgeTreatment(EdgeTreatment),
    CommitEdgeFeature,
    CancelEdgeDraft,
    ToggleEdgeSelectMode,
    ClearEdgeSelection,
    FitCamera,
    Export(String),
    ClearScene,
    StartSketch,
    FinishSketch,
    OpenParams,
    ToggleSection,
}

impl Toolbar {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        show_wireframe: &mut bool,
        show_grid: &mut bool,
        show_measure: &mut bool,
        is_sketching: bool,
        edge_select_mode: bool,
        selected_edge_count: usize,
        edge_treatment: EdgeTreatment,
        active_edge_feature_count: usize,
    ) -> ToolbarAction {
        let t = Tokens::dark();
        let mut action = ToolbarAction::None;

        // 1. Workspace Tabs
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            let tabs = [
                (WorkspaceTab::Solid, "SOLID"),
                (WorkspaceTab::Surface, "SURFACE"),
                (WorkspaceTab::Mesh, "MESH"),
                (WorkspaceTab::SheetMetal, "SHEET METAL"),
                (WorkspaceTab::Utilities, "UTILITIES"),
            ];

            for (tab, label) in tabs {
                let is_active = self.current_tab == tab;
                let text = if is_active {
                    RichText::new(label).strong().color(Color32::WHITE)
                } else {
                    RichText::new(label).color(t.text_dim)
                };

                let resp = ui.selectable_label(is_active, text);
                if resp.clicked() {
                    self.current_tab = tab;
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                if is_sketching {
                    ui.label(RichText::new("✏ SKETCH MODE ACTIVE").color(Color32::from_rgb(255, 200, 80)).strong().size(11.0));
                } else {
                    ui.label(RichText::new("USD-BRep Studio").size(11.0).color(t.text_dim));
                }
            });
        });

        ui.add_space(2.0);
        let sep_stroke = Stroke::new(1.0_f32, t.border);
        let r = ui.available_rect_before_wrap();
        ui.painter().line_segment([r.left_top(), r.right_top()], sep_stroke);
        ui.add_space(4.0);

        // 2. Action Ribbon for Active Tab
        ui.horizontal(|ui| {
            ui.add_space(6.0);

            // Group: SKETCH & CREATE
            ui.group(|ui| {
                ui.label(RichText::new("SKETCH & CREATE").size(9.0).color(t.text_dim));
                ui.separator();

                if is_sketching {
                    let finish_btn = egui::Button::new(RichText::new("✓ Finish Sketch").strong().color(Color32::WHITE))
                        .fill(Color32::from_rgb(46, 150, 90));
                    if ui.add(finish_btn).on_hover_text("Extrude sketch to 3D solid").clicked() {
                        action = ToolbarAction::FinishSketch;
                    }
                } else {
                    let sketch_btn = egui::Button::new(RichText::new("✏ Create Sketch").strong())
                        .fill(t.accent_soft);
                    if ui.add(sketch_btn).on_hover_text("Open 2D sketch canvas on plane").clicked() {
                        action = ToolbarAction::StartSketch;
                    }
                }

                ui.separator();

                if ui.button("📦 Box").on_hover_text("Create a parametric box solid").clicked() {
                    action = ToolbarAction::AddShape(PrimitiveKind::Box);
                }
                if ui.button("⭕ Cyl").on_hover_text("Create a parametric cylinder solid").clicked() {
                    action = ToolbarAction::AddShape(PrimitiveKind::Cylinder);
                }
                if ui.button("🌐 Sphere").on_hover_text("Create a parametric sphere solid").clicked() {
                    action = ToolbarAction::AddShape(PrimitiveKind::Sphere);
                }
                if ui.button("🔺 Cone").on_hover_text("Create a parametric cone solid").clicked() {
                    action = ToolbarAction::AddShape(PrimitiveKind::Cone);
                }
                if ui.button("🍩 Torus").on_hover_text("Create a parametric torus solid").clicked() {
                    action = ToolbarAction::AddShape(PrimitiveKind::Torus);
                }
            });

            ui.add_space(4.0);

            // Group: MODIFY
            ui.group(|ui| {
                let is_drafting = edge_treatment != EdgeTreatment::None;

                let modify_label = if active_edge_feature_count > 0 {
                    format!("MODIFY ({active_edge_feature_count} edge feature(s))")
                } else {
                    "MODIFY".to_string()
                };
                ui.label(RichText::new(modify_label).size(9.0).color(t.text_dim));
                ui.separator();
                if ui.button("⚙ Params").on_hover_text("Open parametric user parameters table").clicked() {
                    action = ToolbarAction::OpenParams;
                }

                if is_drafting {
                    // Composing a new Fillet/Chamfer feature: commit it or discard it.
                    let commit_btn = egui::Button::new(RichText::new("✓ Add Feature").strong().color(Color32::WHITE))
                        .fill(Color32::from_rgb(46, 150, 90));
                    if ui.add(commit_btn).on_hover_text("Add this Fillet/Chamfer as its own feature in the timeline").clicked() {
                        action = ToolbarAction::CommitEdgeFeature;
                    }
                    if ui.button("✕ Cancel").on_hover_text("Discard this in-progress Fillet/Chamfer").clicked() {
                        action = ToolbarAction::CancelEdgeDraft;
                    }
                } else {
                    if ui.button("🧊 Fillet").on_hover_text("Start a new Fillet feature").clicked() {
                        action = ToolbarAction::SetEdgeTreatment(EdgeTreatment::Fillet);
                    }
                    if ui.button("📐 Chamfer").on_hover_text("Start a new Chamfer feature").clicked() {
                        action = ToolbarAction::SetEdgeTreatment(EdgeTreatment::Chamfer);
                    }
                }

                ui.separator();

                let select_label = if edge_select_mode {
                    format!("🎯 Picking Edges ({selected_edge_count})")
                } else {
                    "🎯 Select Edges".to_string()
                };
                let select_btn = egui::Button::new(RichText::new(select_label))
                    .fill(if edge_select_mode { t.accent } else { t.panel_header });
                if ui.add_enabled(is_drafting, select_btn)
                    .on_hover_text("Click edges in the viewport to target this feature at specific edges only. Leave empty to apply to all edges.")
                    .clicked()
                {
                    action = ToolbarAction::ToggleEdgeSelectMode;
                }
                if selected_edge_count > 0 && ui.button("✕ Clear").on_hover_text("Clear edge selection (feature reverts to all edges)").clicked() {
                    action = ToolbarAction::ClearEdgeSelection;
                }
            });

            ui.add_space(4.0);

            // Group: VIEW & DISPLAY
            ui.group(|ui| {
                ui.label(RichText::new("VIEW & INSPECT").size(9.0).color(t.text_dim));
                ui.separator();
                if ui.button("📐 Fit (F)").on_hover_text("Fit all solids into view").clicked() {
                    action = ToolbarAction::FitCamera;
                }
                if ui.button("📊 Section").on_hover_text("Toggle 3D cross-section analysis").clicked() {
                    action = ToolbarAction::ToggleSection;
                }
                ui.checkbox(show_measure, "📏 Measure");
                ui.checkbox(show_wireframe, "Edges");
                ui.checkbox(show_grid, "Grid");
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                egui::menu::menu_button(ui, "📤 Export", |ui| {
                    if ui.button("STEP (.step) - CAD Standard").clicked() {
                        action = ToolbarAction::Export("step".to_string());
                        ui.close_menu();
                    }
                    if ui.button("GLB (.glb) - 3D Web / AR").clicked() {
                        action = ToolbarAction::Export("glb".to_string());
                        ui.close_menu();
                    }
                    if ui.button("STL (.stl) - 3D Printing").clicked() {
                        action = ToolbarAction::Export("stl".to_string());
                        ui.close_menu();
                    }
                });
            });
        });

        ui.add_space(4.0);
        action
    }
}
