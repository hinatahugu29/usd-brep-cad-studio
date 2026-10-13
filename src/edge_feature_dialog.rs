use crate::kernel::EdgeTreatment;
use crate::theme::Tokens;
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke};

pub enum EdgeFeatureDialogAction {
    None,
    Commit,
    Cancel,
}

/// Floating window for composing a new Fillet/Chamfer feature: shown whenever a draft
/// is in progress (`edge_treatment != None`), independent of whether any primitive
/// happens to be selected. Live-previews on the model as the radius/selection change;
/// nothing is added to the feature timeline until "Add Feature" is clicked.
pub struct EdgeFeatureDialog;

impl EdgeFeatureDialog {
    pub fn show(
        ctx: &egui::Context,
        treatment: EdgeTreatment,
        amount: &mut f64,
        selected_edge_count: usize,
        edge_select_mode: &mut bool,
        needs_rebuild: &mut bool,
    ) -> EdgeFeatureDialogAction {
        let t = Tokens::dark();
        let mut result = EdgeFeatureDialogAction::None;

        egui::Window::new(RichText::new(format!("New {} Feature", treatment.display_name())).strong())
            .id(egui::Id::new("edge_feature_dialog"))
            .default_pos([340.0, 110.0])
            .default_width(260.0)
            .resizable(false)
            .collapsible(false)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(t.panel)
                    .stroke(Stroke::new(1.0_f32, t.accent))
                    .corner_radius(CornerRadius::same(6)),
            )
            .show(ctx, |ui| {
                let label = if treatment == EdgeTreatment::Fillet { "Radius (mm)" } else { "Length (mm)" };
                if ui.add(egui::Slider::new(amount, 0.1..=50.0).text(label)).changed() {
                    *needs_rebuild = true;
                }

                ui.add_space(6.0);
                if selected_edge_count > 0 {
                    ui.label(
                        RichText::new(format!("🎯 Targeting {selected_edge_count} selected edge(s)"))
                            .size(11.0)
                            .color(Color32::from_rgb(255, 196, 60)),
                    );
                } else {
                    ui.label(
                        RichText::new("Applies to ALL edges — use 🎯 Select Edges to target specific ones")
                            .size(11.0)
                            .color(t.text_dim),
                    );
                }

                ui.checkbox(edge_select_mode, "🎯 Pick edges in viewport");

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    let commit_btn = egui::Button::new(RichText::new("✓ Add Feature").strong().color(Color32::WHITE))
                        .fill(Color32::from_rgb(46, 150, 90));
                    if ui.add(commit_btn).clicked() {
                        result = EdgeFeatureDialogAction::Commit;
                    }
                    if ui.button("✕ Cancel").clicked() {
                        result = EdgeFeatureDialogAction::Cancel;
                    }
                });
            });

        result
    }
}
