use crate::theme::Tokens;
use eframe::egui::{self, CornerRadius, RichText, Stroke};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SectionAxis {
    X,
    Y,
    Z,
}

pub struct SectionAnalysis {
    pub is_enabled: bool,
    pub axis: SectionAxis,
    pub offset: f32,
    pub flipped: bool,
}

impl Default for SectionAnalysis {
    fn default() -> Self {
        Self {
            is_enabled: false,
            axis: SectionAxis::Z,
            offset: 0.0,
            flipped: false,
        }
    }
}

impl SectionAnalysis {
    /// Returns the clip plane equation (nx, ny, nz, d) where dot(pos, n) + d >= 0 is kept
    pub fn clip_plane_equation(&self) -> Option<[f32; 4]> {
        if !self.is_enabled {
            return None;
        }

        let sign = if self.flipped { -1.0_f32 } else { 1.0_f32 };
        match self.axis {
            SectionAxis::X => Some([sign, 0.0, 0.0, -sign * self.offset]),
            SectionAxis::Y => Some([0.0, sign, 0.0, -sign * self.offset]),
            SectionAxis::Z => Some([0.0, 0.0, sign, -sign * self.offset]),
        }
    }

    pub fn show_dialog(&mut self, ctx: &egui::Context) {
        if !self.is_enabled {
            return;
        }

        let t = Tokens::dark();

        egui::Window::new(RichText::new("📊 Section Analysis").strong())
            .id(egui::Id::new("section_analysis_dialog"))
            .default_pos([340.0, 480.0])
            .default_width(260.0)
            .resizable(false)
            .collapsible(true)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(t.panel)
                    .stroke(Stroke::new(1.0_f32, t.border))
                    .corner_radius(CornerRadius::same(6)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Cut Plane:");
                    if ui.selectable_value(&mut self.axis, SectionAxis::Z, "XY (Z)").clicked() {}
                    if ui.selectable_value(&mut self.axis, SectionAxis::Y, "XZ (Y)").clicked() {}
                    if ui.selectable_value(&mut self.axis, SectionAxis::X, "YZ (X)").clicked() {}
                });

                ui.add_space(4.0);
                ui.add(egui::Slider::new(&mut self.offset, -80.0..=80.0).text("Offset (mm)"));

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.flipped, "Flip Normal");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✕ Disable").clicked() {
                            self.is_enabled = false;
                        }
                    });
                });
            });
    }
}
