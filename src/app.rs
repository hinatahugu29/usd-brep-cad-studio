use crate::browser::Browser;
use crate::camera::Camera;
use crate::dialogs::FeatureDialog;
use crate::edge_feature_dialog::{EdgeFeatureDialog, EdgeFeatureDialogAction};
use crate::edge_picker;
use crate::gizmo::Gizmo;
use crate::kernel::{self, BoolOp, EdgeFeature, EdgeTreatment, EvaluationResult, PrimitiveKind, ShapeDef};
use crate::measure::MeasureTool;
use crate::params::UserParams;
use crate::primitive_bounds;
use crate::renderer::CadRenderer;
use crate::section_analysis::SectionAnalysis;
use crate::shortcut_box::ShortcutBox;
use crate::sketch::SketchSession;
use crate::sketch_palette::SketchPalette;
use crate::theme::{self, Tokens};
use crate::timeline::Timeline;
use crate::toolbar::{Toolbar, ToolbarAction};
use crate::viewcube::ViewCube;
use eframe::egui::{self, Align2, Color32, FontId, Key, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use glam::{Mat4, Vec3};
use std::sync::{Arc, Mutex};

pub struct CadApp {
    shapes: Vec<ShapeDef>,
    selected_id: Option<u64>,
    next_id: u64,
    rollback_index: usize,
    camera: Camera,
    renderer: Option<Arc<Mutex<CadRenderer>>>,
    eval_result: EvaluationResult,
    needs_rebuild: bool,
    show_wireframe: bool,
    show_grid: bool,
    status_message: String,
    /// Draft state for a NOT-YET-COMMITTED Fillet/Chamfer feature being composed
    /// interactively (live-previewed on top of the committed `edge_features`).
    edge_treatment: EdgeTreatment,
    edge_amount: f64,
    selected_edges: Vec<usize>,
    edge_select_mode: bool,
    hover_edge: Option<usize>,
    /// Committed Fillet/Chamfer feature history, applied in order after all
    /// shapes/booleans — each its own reorderable, suppressible, deletable feature.
    edge_features: Vec<EdgeFeature>,
    next_edge_feature_id: u64,
    browser: Browser,
    toolbar: Toolbar,
    gizmo: Gizmo,
    sketch_session: SketchSession,
    shortcut_box: ShortcutBox,
    measure_tool: MeasureTool,
    user_params: UserParams,
    section_analysis: SectionAnalysis,
}

impl CadApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Apply USD-BRep CAD Studio's dark Nord theme
        theme::apply_theme(&cc.egui_ctx);

        // Load system CJK font fallback if available
        let mut fonts = egui::FontDefinitions::default();
        for path in &[
            "C:\\Windows\\Fonts\\meiryo.ttc",
            "C:\\Windows\\Fonts\\YuGothM.ttc",
            "C:\\Windows\\Fonts\\msgothic.ttc",
        ] {
            if let Ok(data) = std::fs::read(path) {
                fonts.font_data.insert(
                    "jp_font".to_owned(),
                    Arc::new(egui::FontData::from_owned(data)),
                );
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .push("jp_font".to_owned());
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .push("jp_font".to_owned());
                break;
            }
        }
        cc.egui_ctx.set_fonts(fonts);

        let renderer = if let Some(gl) = &cc.gl {
            match CadRenderer::new(gl.clone()) {
                Ok(r) => Some(Arc::new(Mutex::new(r))),
                Err(e) => {
                    eprintln!("Failed to initialize 3D renderer: {e}");
                    None
                }
            }
        } else {
            None
        };

        // Create default starter CAD model: Base Box with a cut Hole Cylinder
        let mut shapes = Vec::new();
        let box_shape = ShapeDef::new_box(1, "Base Box", 30.0, 30.0, 15.0);
        let mut cyl_shape = ShapeDef::new_cylinder(2, "Hole Cylinder", 8.0, 20.0);
        cyl_shape.op = BoolOp::Cut;

        shapes.push(box_shape);
        shapes.push(cyl_shape);
        let rollback_index = shapes.len();

        let mut app = Self {
            shapes,
            selected_id: Some(1),
            next_id: 3,
            rollback_index,
            camera: Camera::default(),
            renderer,
            eval_result: EvaluationResult::default(),
            needs_rebuild: true,
            show_wireframe: true,
            show_grid: true,
            status_message: "Ready. USD-BRep SMLib parametric kernel active.".to_string(),
            edge_treatment: EdgeTreatment::None,
            edge_amount: 2.0,
            selected_edges: Vec::new(),
            edge_select_mode: false,
            hover_edge: None,
            edge_features: Vec::new(),
            next_edge_feature_id: 1,
            browser: Browser::default(),
            toolbar: Toolbar::default(),
            gizmo: Gizmo::default(),
            sketch_session: SketchSession::default(),
            shortcut_box: ShortcutBox::default(),
            measure_tool: MeasureTool::default(),
            user_params: UserParams::default(),
            section_analysis: SectionAnalysis::default(),
        };

        app.rebuild_model();
        app
    }

    /// Total combined length of the timeline: shape/boolean features followed by
    /// Fillet/Chamfer edge features. `rollback_index` ranges over `0..=history_len()`.
    fn history_len(&self) -> usize {
        self.shapes.len() + self.edge_features.len()
    }

    /// Edge features currently active per `rollback_index` (i.e. not rolled back past).
    fn active_edge_features(&self) -> &[EdgeFeature] {
        let n = self.rollback_index.saturating_sub(self.shapes.len()).min(self.edge_features.len());
        &self.edge_features[0..n]
    }

    /// The in-progress draft feature, if the user is currently composing one.
    fn draft_edge_feature(&self) -> Option<EdgeFeature> {
        if self.edge_treatment == EdgeTreatment::None {
            return None;
        }
        Some(EdgeFeature::new(0, self.edge_treatment, self.edge_amount, self.selected_edges.clone()))
    }

    fn rebuild_model(&mut self) {
        let active_slice = &self.shapes[0..self.rollback_index.min(self.shapes.len())];
        let draft = self.draft_edge_feature();
        self.eval_result = kernel::evaluate_scene(active_slice, self.active_edge_features(), draft.as_ref());
        // Edge indices only refer to the current composite's topology; drop any that
        // fell out of range (e.g. after changing a shape's geometry).
        let edge_count = self.eval_result.edge_count;
        self.selected_edges.retain(|&i| i < edge_count);
        if let Some(err) = &self.eval_result.error {
            self.status_message = format!("Kernel error: {err}");
        } else {
            let feature_scope = if !self.edge_features.is_empty() {
                format!(" | {} edge feature(s)", self.active_edge_features().len())
            } else {
                String::new()
            };
            self.status_message = format!(
                "Rebuild OK | Triangles: {} | Volume: {:.2} mm³ | Surface: {:.2} mm²{feature_scope}",
                self.eval_result.triangle_count, self.eval_result.volume, self.eval_result.surface_area
            );
            if let Some(warn) = &self.eval_result.warning {
                self.status_message = format!("{} | ⚠ {}", self.status_message, warn);
            }
            if let Some(renderer_arc) = &self.renderer {
                if let Ok(mut r) = renderer_arc.lock() {
                    r.update_mesh_data(
                        &self.eval_result.vertex_buffer,
                        &self.eval_result.line_buffer,
                    );
                }
            }
        }
        self.needs_rebuild = false;
    }

    /// Commits the in-progress draft Fillet/Chamfer as a standalone, reorderable
    /// feature at the end of the active history, then resets the draft state.
    fn commit_edge_feature(&mut self) {
        if self.edge_treatment == EdgeTreatment::None {
            return;
        }
        let id = self.next_edge_feature_id;
        self.next_edge_feature_id += 1;
        let feature = EdgeFeature::new(id, self.edge_treatment, self.edge_amount, std::mem::take(&mut self.selected_edges));

        // Insert at the currently active position (so committing mid-rollback doesn't
        // silently discard suppressed later features — it inserts right after what's active).
        let insert_at = self.rollback_index.saturating_sub(self.shapes.len()).min(self.edge_features.len());
        self.edge_features.insert(insert_at, feature);
        self.rollback_index = self.shapes.len() + insert_at + 1;

        self.edge_treatment = EdgeTreatment::None;
        self.edge_select_mode = false;
        self.hover_edge = None;
        self.needs_rebuild = true;
    }

    /// Discards the in-progress draft without creating a feature.
    fn cancel_edge_draft(&mut self) {
        self.edge_treatment = EdgeTreatment::None;
        self.selected_edges.clear();
        self.edge_select_mode = false;
        self.hover_edge = None;
        self.needs_rebuild = true;
    }

    fn clear_scene(&mut self) {
        self.shapes.clear();
        self.edge_features.clear();
        self.selected_id = None;
        self.rollback_index = 0;
        self.cancel_edge_draft();
    }

    fn add_shape(&mut self, kind: PrimitiveKind) {
        let id = self.next_id;
        self.next_id += 1;

        let shape = match kind {
            PrimitiveKind::Box => ShapeDef::new_box(id, &format!("Box {id}"), 20.0, 20.0, 20.0),
            PrimitiveKind::Cylinder => ShapeDef::new_cylinder(id, &format!("Cylinder {id}"), 6.0, 25.0),
            PrimitiveKind::Sphere => ShapeDef::new_sphere(id, &format!("Sphere {id}"), 10.0),
            PrimitiveKind::Cone => ShapeDef::new_cone(id, &format!("Cone {id}"), 10.0, 0.0, 15.0),
            PrimitiveKind::Torus => ShapeDef::new_torus(id, &format!("Torus {id}"), 15.0, 4.0),
        };

        self.shapes.push(shape);
        self.selected_id = Some(id);
        self.rollback_index = self.history_len();
        self.needs_rebuild = true;
    }

    fn duplicate_selected(&mut self) {
        let Some(sel_id) = self.selected_id else { return };
        if let Some(orig) = self.shapes.iter().find(|s| s.id == sel_id).cloned() {
            let new_id = self.next_id;
            self.next_id += 1;
            let mut copy = orig;
            copy.id = new_id;
            copy.name = format!("{} (Copy)", copy.name);
            copy.position[0] += 15.0; // Offset copy
            self.shapes.push(copy);
            self.selected_id = Some(new_id);
            self.rollback_index = self.history_len();
            self.needs_rebuild = true;
        }
    }

    fn export_file(&mut self, fmt: &str) {
        let ext = match fmt {
            "step" => "step",
            "glb" => "glb",
            "stl" => "stl",
            _ => "step",
        };

        let file = rfd::FileDialog::new()
            .set_file_name(&format!("model.{ext}"))
            .add_filter(&format!("{fmt} file"), &[ext])
            .save_file();

        if let Some(path_buf) = file {
            let path_str = path_buf.to_string_lossy().to_string();
            let active_slice = &self.shapes[0..self.rollback_index.min(self.shapes.len())];
            match kernel::export_scene(active_slice, fmt, &path_str, self.active_edge_features()) {
                Ok(()) => {
                    self.status_message = format!("Successfully exported to {path_str}");
                }
                Err(e) => {
                    self.status_message = format!("Export failed: {e}");
                }
            }
        }
    }

    fn execute_quick_command(&mut self, cmd: &str) {
        match cmd {
            "box" => self.add_shape(PrimitiveKind::Box),
            "cyl" => self.add_shape(PrimitiveKind::Cylinder),
            "sphere" => self.add_shape(PrimitiveKind::Sphere),
            "cone" => self.add_shape(PrimitiveKind::Cone),
            "torus" => self.add_shape(PrimitiveKind::Torus),
            "sketch" => {
                self.sketch_session.is_active = true;
                self.camera.set_preset_view("top");
            }
            "fillet" => {
                self.edge_treatment = EdgeTreatment::Fillet;
                self.needs_rebuild = true;
            }
            "chamfer" => {
                self.edge_treatment = EdgeTreatment::Chamfer;
                self.needs_rebuild = true;
            }
            "measure" => {
                self.measure_tool.show_dimensions = !self.measure_tool.show_dimensions;
            }
            "fit" => self.camera.fit_to_bbox(self.eval_result.bbox_min, self.eval_result.bbox_max),
            "iso" => self.camera.set_preset_view("iso"),
            "top" => self.camera.set_preset_view("top"),
            "front" => self.camera.set_preset_view("front"),
            "right" => self.camera.set_preset_view("right"),
            "step" => self.export_file("step"),
            "glb" => self.export_file("glb"),
            "stl" => self.export_file("stl"),
            "params" => self.user_params.is_open = true,
            "section" => self.section_analysis.is_enabled = !self.section_analysis.is_enabled,
            "clear" => self.clear_scene(),
            _ => {}
        }
    }
}

impl eframe::App for CadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let t = Tokens::dark();

        if self.needs_rebuild {
            self.rebuild_model();
        }

        // Global 'S' shortcut to open command search box at cursor
        if !self.shortcut_box.is_open && ctx.input(|i| i.key_pressed(Key::S)) {
            let cursor = ctx.input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::new(400.0, 300.0));
            self.shortcut_box.open_at(cursor);
        }

        // 1. Top Application Bar & Main Menu
        egui::TopBottomPanel::top("app_top_bar")
            .frame(egui::Frame::NONE.fill(t.app_bar))
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.label(RichText::new("USD-BRep CAD Studio").strong().color(Color32::WHITE));
                    ui.separator();

                    ui.menu_button("File", |ui| {
                        if ui.button("New Design").clicked() {
                            self.clear_scene();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Export STEP (AP242 CAD standard)").clicked() {
                            self.export_file("step");
                            ui.close_menu();
                        }
                        if ui.button("Export GLB (3D web/viewer)").clicked() {
                            self.export_file("glb");
                            ui.close_menu();
                        }
                        if ui.button("Export STL (3D printing)").clicked() {
                            self.export_file("stl");
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Edit", |ui| {
                        if ui.button("Duplicate Selected (Ctrl+D)").clicked() {
                            self.duplicate_selected();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Clear Scene").clicked() {
                            self.clear_scene();
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("View", |ui| {
                        if ui.button("Reset Camera (F)").clicked() {
                            self.camera.fit_to_bbox(self.eval_result.bbox_min, self.eval_result.bbox_max);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Isometric View").clicked() {
                            self.camera.set_preset_view("iso");
                            ui.close_menu();
                        }
                        if ui.button("Top View").clicked() {
                            self.camera.set_preset_view("top");
                            ui.close_menu();
                        }
                        if ui.button("Front View").clicked() {
                            self.camera.set_preset_view("front");
                            ui.close_menu();
                        }
                        if ui.button("Right View").clicked() {
                            self.camera.set_preset_view("right");
                            ui.close_menu();
                        }
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new("Press 'S' for Quick Search | 'F' to Fit").size(11.0).color(t.text_dim));
                    });
                });
            });

        // 2. Ribbon Toolbar Panel
        egui::TopBottomPanel::top("ribbon_toolbar")
            .frame(egui::Frame::NONE.fill(t.toolbar))
            .show(ctx, |ui| {
                let active_edge_feature_count = self.active_edge_features().len();
                let action = self.toolbar.show(
                    ui,
                    &mut self.show_wireframe,
                    &mut self.show_grid,
                    &mut self.measure_tool.show_dimensions,
                    self.sketch_session.is_active,
                    self.edge_select_mode,
                    self.selected_edges.len(),
                    self.edge_treatment,
                    active_edge_feature_count,
                );
                match action {
                    ToolbarAction::None => {}
                    ToolbarAction::ToggleEdgeSelectMode => {
                        self.edge_select_mode = !self.edge_select_mode;
                    }
                    ToolbarAction::ClearEdgeSelection => {
                        self.selected_edges.clear();
                        self.needs_rebuild = true;
                    }
                    ToolbarAction::StartSketch => {
                        self.sketch_session.is_active = true;
                        self.camera.set_preset_view("top");
                    }
                    ToolbarAction::FinishSketch => {
                        let new_solids = self.sketch_session.extrude_to_shapes(&mut self.next_id);
                        if !new_solids.is_empty() {
                            self.shapes.extend(new_solids);
                            self.selected_id = self.shapes.last().map(|s| s.id);
                            self.rollback_index = self.history_len();
                            self.needs_rebuild = true;
                        }
                    }
                    ToolbarAction::AddShape(kind) => {
                        self.add_shape(kind);
                    }
                    ToolbarAction::SetEdgeTreatment(edge_op) => {
                        self.edge_treatment = edge_op;
                        self.needs_rebuild = true;
                    }
                    ToolbarAction::CommitEdgeFeature => {
                        self.commit_edge_feature();
                    }
                    ToolbarAction::CancelEdgeDraft => {
                        self.cancel_edge_draft();
                    }
                    ToolbarAction::FitCamera => {
                        self.camera.fit_to_bbox(self.eval_result.bbox_min, self.eval_result.bbox_max);
                    }
                    ToolbarAction::Export(fmt) => {
                        self.export_file(&fmt);
                    }
                    ToolbarAction::OpenParams => {
                        self.user_params.is_open = true;
                    }
                    ToolbarAction::ToggleSection => {
                        self.section_analysis.is_enabled = !self.section_analysis.is_enabled;
                    }
                    ToolbarAction::ClearScene => {
                        self.clear_scene();
                    }
                }
            });

        // 3. Bottom Status Bar
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(24.0)
            .frame(egui::Frame::NONE.fill(t.app_bar))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new("●").color(Color32::from_rgb(90, 200, 120)).size(10.0));
                    ui.label(RichText::new(&self.status_message).size(11.5).color(t.text_dim));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        ui.label(RichText::new("USD-BRep Direct Modeling + SMLib").size(11.0).color(t.accent));
                    });
                });
            });

        // 4. Bottom Parametric Timeline
        Timeline::show(
            ctx,
            &mut self.shapes,
            &mut self.edge_features,
            &mut self.selected_id,
            &mut self.rollback_index,
            &mut self.needs_rebuild,
        );

        // 5. Left Model Browser Panel
        egui::SidePanel::left("browser_panel")
            .resizable(true)
            .default_width(260.0)
            .min_width(200.0)
            .max_width(450.0)
            .frame(egui::Frame::NONE.fill(t.panel).stroke(Stroke::new(1.0_f32, t.border)))
            .show(ctx, |ui| {
                self.browser.show(
                    ui,
                    &mut self.shapes,
                    &mut self.selected_id,
                    &self.eval_result,
                    &mut self.camera,
                    &mut self.needs_rebuild,
                );
            });

        // 6. Floating Feature Dialog (Properties Inspector)
        FeatureDialog::show(ctx, &mut self.shapes, self.selected_id, &mut self.needs_rebuild);

        // 6a. Floating Edge Feature Composer (Fillet/Chamfer draft, shown while composing)
        if self.edge_treatment != EdgeTreatment::None {
            let commit = EdgeFeatureDialog::show(
                ctx,
                self.edge_treatment,
                &mut self.edge_amount,
                self.selected_edges.len(),
                &mut self.edge_select_mode,
                &mut self.needs_rebuild,
            );
            match commit {
                EdgeFeatureDialogAction::None => {}
                EdgeFeatureDialogAction::Commit => self.commit_edge_feature(),
                EdgeFeatureDialogAction::Cancel => self.cancel_edge_draft(),
            }
        }

        // 6b. Fusion 360-style Parameters Dialog (User Variables Table)
        self.user_params.show_dialog(ctx, &mut self.shapes, &mut self.needs_rebuild);

        // 6c. Section Analysis Dialog (Slice plane)
        self.section_analysis.show_dialog(ctx);

        // 7. Floating Sketch Palette (when sketch mode is active)
        let mut finish_extrude = false;
        SketchPalette::show(
            ctx,
            &mut self.sketch_session,
            &mut self.camera,
            &mut finish_extrude,
        );
        if finish_extrude {
            let new_solids = self.sketch_session.extrude_to_shapes(&mut self.next_id);
            if !new_solids.is_empty() {
                self.shapes.extend(new_solids);
                self.selected_id = self.shapes.last().map(|s| s.id);
                self.rollback_index = self.history_len();
                self.needs_rebuild = true;
            }
        }

        // 8. S-Key Command Search Box (Shortcut Box)
        if let Some(cmd) = self.shortcut_box.show(ctx) {
            self.execute_quick_command(cmd);
        }

        // 9. Central 3D Viewport
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(t.viewport_bg))
            .show(ctx, |ui| {
                let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());

                // Scroll zoom
                let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll_delta != 0.0 && response.hovered() {
                    let zoom_factor = if scroll_delta > 0.0 { 0.90 } else { 1.11 };
                    self.camera.zoom(zoom_factor);
                }

                // Keyboard shortcut 'F' to fit
                if ui.input(|i| i.key_pressed(Key::F)) {
                    self.camera.fit_to_bbox(self.eval_result.bbox_min, self.eval_result.bbox_max);
                }

                // Arrow-key nudge of the selected primitive's position (skipped while
                // typing into a text field, sketching, or picking edges).
                if !ctx.wants_keyboard_input()
                    && !self.sketch_session.is_active
                    && !self.edge_select_mode
                {
                    if let Some(sel_id) = self.selected_id {
                        if let Some(shape) = self.shapes.iter_mut().find(|s| s.id == sel_id) {
                            let shift = ui.input(|i| i.modifiers.shift);
                            let step = if shift { 5.0 } else { 0.5 };
                            let mut delta = [0.0_f64; 3];
                            ui.input(|i| {
                                if i.key_pressed(Key::ArrowLeft) { delta[0] -= step; }
                                if i.key_pressed(Key::ArrowRight) { delta[0] += step; }
                                if i.key_pressed(Key::ArrowUp) { delta[1] += step; }
                                if i.key_pressed(Key::ArrowDown) { delta[1] -= step; }
                                if i.key_pressed(Key::PageUp) { delta[2] += step; }
                                if i.key_pressed(Key::PageDown) { delta[2] -= step; }
                            });
                            if delta != [0.0; 3] {
                                shape.position[0] += delta[0];
                                shape.position[1] += delta[1];
                                shape.position[2] += delta[2];
                                self.needs_rebuild = true;
                            }
                        }
                    }
                }

                // 3D OpenGL Rendering Callback
                if let Some(renderer_arc) = self.renderer.clone() {
                    let view = self.camera.view_matrix();
                    let show_wireframe = self.show_wireframe;
                    let show_grid = self.show_grid;
                    let camera = self.camera.clone();
                    let clip_plane = self.section_analysis.clip_plane_equation();

                    // Determine active solid color (if selected, use its appearance color)
                    let active_color = self
                        .selected_id
                        .and_then(|id| self.shapes.iter().find(|s| s.id == id).map(|s| s.color))
                        .unwrap_or([0.52, 0.65, 0.82]);

                    let callback = egui_glow::CallbackFn::new(move |info, _painter| {
                        let vp = info.viewport_in_pixels();
                        let aspect = (vp.width_px as f32) / (vp.height_px.max(1) as f32);
                        let proj = camera.proj_matrix(aspect);
                        let mvp = proj * view;
                        let model = Mat4::IDENTITY;

                        if let Ok(r) = renderer_arc.lock() {
                            r.render(
                                vp.left_px,
                                vp.from_bottom_px,
                                vp.width_px,
                                vp.height_px,
                                &mvp,
                                &model,
                                active_color,
                                clip_plane,
                                show_wireframe,
                                show_grid,
                            );
                        }
                    });

                    ui.painter().add(egui::PaintCallback {
                        rect,
                        callback: Arc::new(callback),
                    });
                }

                // Viewport HUD Overlay & Controls
                let painter = ui.painter_at(rect);
                painter.rect_stroke(
                    rect,
                    0.0,
                    Stroke::new(1.0_f32, t.border),
                    egui::StrokeKind::Inside,
                );

                // Edge pick/highlight math shared between interaction modes below
                let aspect_pick = (rect.width() / rect.height().max(1.0)) as f32;
                let mvp_pick = self.camera.proj_matrix(aspect_pick) * self.camera.view_matrix();
                let pickable_edges = edge_picker::decode_edges(&self.eval_result.edge_points);

                // Handling Interactions: Sketch Canvas vs Edge Picking vs Direct 3D Gizmo vs Camera
                if self.sketch_session.is_active {
                    // 1. Sketch interaction
                    self.sketch_session.handle_canvas(ui, &self.camera, rect);

                    // Allow Pan in sketch mode with secondary/middle drag
                    if response.dragged_by(egui::PointerButton::Secondary)
                        || response.dragged_by(egui::PointerButton::Middle)
                    {
                        let delta = response.drag_delta();
                        self.camera.pan(delta.x, delta.y);
                    }
                } else if self.edge_select_mode {
                    // Edge Select Mode: hover + click-to-toggle targeted Fillet/Chamfer edges.
                    // Camera orbit/pan still work via drag; a plain click toggles the hovered edge.
                    self.hover_edge = response
                        .hover_pos()
                        .and_then(|p| edge_picker::pick_edge(p, &mvp_pick, rect, &pickable_edges, 8.0));

                    if response.clicked() {
                        if let Some(idx) = self.hover_edge {
                            if let Some(pos) = self.selected_edges.iter().position(|&e| e == idx) {
                                self.selected_edges.remove(pos);
                            } else {
                                self.selected_edges.push(idx);
                            }
                            self.needs_rebuild = true;
                        }
                    }

                    if response.dragged_by(egui::PointerButton::Primary) {
                        let delta = response.drag_delta();
                        self.camera.orbit(delta.x, delta.y);
                    } else if response.dragged_by(egui::PointerButton::Secondary)
                        || response.dragged_by(egui::PointerButton::Middle)
                    {
                        let delta = response.drag_delta();
                        self.camera.pan(delta.x, delta.y);
                    }

                    edge_picker::draw_highlight_overlay(
                        ui,
                        rect,
                        &mvp_pick,
                        &pickable_edges,
                        &self.selected_edges,
                        self.hover_edge,
                    );
                } else {
                    self.hover_edge = None;
                    if !self.selected_edges.is_empty() {
                        edge_picker::draw_highlight_overlay(
                            ui,
                            rect,
                            &mvp_pick,
                            &pickable_edges,
                            &self.selected_edges,
                            None,
                        );
                    }
                    // 2. Direct 3D Gizmo manipulation
                    let mut gizmo_manipulated = false;
                    if let Some(sel_id) = self.selected_id {
                        if let Some(shape) = self.shapes.iter_mut().find(|s| s.id == sel_id) {
                            gizmo_manipulated = self.gizmo.interact(
                                ui,
                                &self.camera,
                                rect,
                                &mut shape.position,
                                &mut self.needs_rebuild,
                            );
                        }
                    }

                    // 2b. Ghost outline for the selected primitive: translucent box
                    // showing its true footprint while moving, red + warning text if a
                    // Cut (hole) primitive no longer overlaps the solid material it's
                    // meant to remove (i.e. it would have no visible effect).
                    if let Some(sel_id) = self.selected_id {
                        if let Some(idx) = self.shapes.iter().position(|s| s.id == sel_id) {
                            if idx < self.rollback_index {
                                let shape = &self.shapes[idx];
                                let aabb = primitive_bounds::world_aabb(
                                    shape.kind, shape.params, shape.rotation, shape.position,
                                );
                                let is_cut = shape.op == BoolOp::Cut;

                                let overlaps_prior = if is_cut {
                                    let mut acc: Option<(Vec3, Vec3)> = None;
                                    for prior in &self.shapes[0..idx] {
                                        if !prior.visible { continue; }
                                        let pb = primitive_bounds::world_aabb(
                                            prior.kind, prior.params, prior.rotation, prior.position,
                                        );
                                        acc = Some(match acc {
                                            None => pb,
                                            Some((amin, amax)) => (amin.min(pb.0), amax.max(pb.1)),
                                        });
                                    }
                                    acc.is_some_and(|a| primitive_bounds::aabb_intersects(a, aabb))
                                } else {
                                    true
                                };

                                let (color, stroke_w) = if is_cut && !overlaps_prior {
                                    (Color32::from_rgb(255, 70, 70), 2.5_f32)
                                } else if is_cut {
                                    (Color32::from_rgb(255, 160, 70), 1.5_f32)
                                } else {
                                    (Color32::from_rgb(100, 225, 180), 1.5_f32)
                                };

                                for (a, b) in primitive_bounds::aabb_edges(aabb) {
                                    if let (Some(sa), Some(sb)) = (
                                        edge_picker::project_to_screen(a, &mvp_pick, rect),
                                        edge_picker::project_to_screen(b, &mvp_pick, rect),
                                    ) {
                                        painter.line_segment([sa, sb], Stroke::new(stroke_w, color));
                                    }
                                }

                                if is_cut && !overlaps_prior {
                                    let center = (aabb.0 + aabb.1) * 0.5;
                                    if let Some(p) = edge_picker::project_to_screen(center, &mvp_pick, rect) {
                                        painter.text(
                                            p,
                                            Align2::CENTER_CENTER,
                                            "⚠ Cut doesn't touch the solid — no effect",
                                            FontId::proportional(12.0),
                                            Color32::from_rgb(255, 90, 90),
                                        );
                                    }
                                }
                            }
                        }
                    }

                    // 3. Camera Drag Handling (when not grabbing 3D gizmo)
                    if !gizmo_manipulated {
                        if response.dragged_by(egui::PointerButton::Primary) {
                            let delta = response.drag_delta();
                            self.camera.orbit(delta.x, delta.y);
                        } else if response.dragged_by(egui::PointerButton::Secondary)
                            || response.dragged_by(egui::PointerButton::Middle)
                        {
                            let delta = response.drag_delta();
                            self.camera.pan(delta.x, delta.y);
                        }
                    }

                    // 4. Measure Tool Overlay (3D Dimension extension lines)
                    self.measure_tool.render_dimensions(
                        ui,
                        &self.camera,
                        rect,
                        &self.shapes,
                        self.selected_id,
                    );

                    // 5. Right-click Context Menu in Viewport
                    response.context_menu(|ui| {
                        if let Some(sel_id) = self.selected_id {
                            let shape_name = self.shapes.iter().find(|s| s.id == sel_id).map(|s| s.name.clone()).unwrap_or_default();
                            ui.label(RichText::new(format!("Solid: {shape_name}")).strong().color(t.accent));
                            ui.separator();
                            if ui.button("📋 Duplicate (Copy)").clicked() {
                                self.duplicate_selected();
                                ui.close_menu();
                            }
                            if ui.button("🧊 Apply Fillet").clicked() {
                                self.edge_treatment = EdgeTreatment::Fillet;
                                self.needs_rebuild = true;
                                ui.close_menu();
                            }
                            if ui.button("📐 Apply Chamfer").clicked() {
                                self.edge_treatment = EdgeTreatment::Chamfer;
                                self.needs_rebuild = true;
                                ui.close_menu();
                            }
                            if ui.button("🗑 Delete").clicked() {
                                self.shapes.retain(|s| s.id != sel_id);
                                self.selected_id = self.shapes.first().map(|s| s.id);
                                self.rollback_index = self.history_len();
                                self.needs_rebuild = true;
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button("📤 Export STEP").clicked() {
                                self.export_file("step");
                                ui.close_menu();
                            }
                        } else {
                            ui.label("Viewport Quick Menu");
                            ui.separator();
                            if ui.button("📦 Add Box").clicked() {
                                self.add_shape(PrimitiveKind::Box);
                                ui.close_menu();
                            }
                            if ui.button("⭕ Add Cylinder").clicked() {
                                self.add_shape(PrimitiveKind::Cylinder);
                                ui.close_menu();
                            }
                            if ui.button("✏ 2D Sketch").clicked() {
                                self.sketch_session.is_active = true;
                                self.camera.set_preset_view("top");
                                ui.close_menu();
                            }
                        }
                    });
                }

                // Viewport Navigation Bar (Floating chips at bottom-center of viewport)
                let nav_bar_w = 260.0;
                let nav_bar_h = 32.0;
                let nav_rect = Rect::from_center_size(
                    Pos2::new(rect.center().x, rect.bottom() - 24.0),
                    Vec2::new(nav_bar_w, nav_bar_h),
                );

                painter.rect(
                    nav_rect,
                    6.0,
                    Color32::from_rgba_premultiplied(33, 37, 45, 220),
                    Stroke::new(1.0_f32, t.border),
                    egui::StrokeKind::Inside,
                );

                // Navigation Buttons inside HUD
                let hud_actions = [
                    ("📐 Fit", "Fit model to view (F)"),
                    ("⌂ ISO", "Isometric view"),
                    ("🔲 Top", "Top view"),
                    ("🔲 Front", "Front view"),
                ];

                let btn_w = nav_bar_w / hud_actions.len() as f32;
                for (i, (lbl, tooltip)) in hud_actions.iter().enumerate() {
                    let b_rect = Rect::from_min_size(
                        Pos2::new(nav_rect.left() + i as f32 * btn_w, nav_rect.top()),
                        Vec2::new(btn_w, nav_bar_h),
                    );
                    let b_resp = ui.interact(b_rect, ui.id().with(("hud_nav", i)), Sense::click());
                    if b_resp.hovered() {
                        painter.rect_filled(b_rect, 4.0, t.hover);
                    }
                    painter.text(
                        b_rect.center(),
                        Align2::CENTER_CENTER,
                        lbl,
                        FontId::proportional(11.0),
                        Color32::WHITE,
                    );

                    if b_resp.on_hover_text(*tooltip).clicked() {
                        match i {
                            0 => self.camera.fit_to_bbox(self.eval_result.bbox_min, self.eval_result.bbox_max),
                            1 => self.camera.set_preset_view("iso"),
                            2 => self.camera.set_preset_view("top"),
                            3 => self.camera.set_preset_view("front"),
                            _ => {}
                        }
                    }
                }

                // Interactive ViewCube in top-right corner
                ViewCube::show(ui, &mut self.camera, rect);
            });
    }
}
