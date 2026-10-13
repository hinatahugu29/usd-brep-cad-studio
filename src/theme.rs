use eframe::egui::{self, Color32, CornerRadius, Stroke, Vec2};

#[derive(Clone, Copy, Debug)]
pub struct Tokens {
    pub app_bar: Color32,
    pub toolbar: Color32,
    pub panel: Color32,
    pub panel_header: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub viewport_bg: Color32,
    pub grid: Color32,
    pub grid_major: Color32,
    pub timeline: Color32,
    pub timeline_item: Color32,
    pub timeline_marker: Color32,
    pub hover: Color32,
    pub field: Color32,
    pub error: Color32,
    pub warning: Color32,
    pub success: Color32,
}

impl Tokens {
    /// USD-BRep CAD Studio's Nord-inspired CAD dark theme
    pub const fn dark() -> Self {
        Self {
            app_bar: Color32::from_rgb(28, 31, 38),
            toolbar: Color32::from_rgb(37, 41, 49),
            panel: Color32::from_rgb(39, 43, 52),
            panel_header: Color32::from_rgb(47, 52, 62),
            border: Color32::from_rgb(63, 69, 82),
            text: Color32::from_rgb(222, 226, 232),
            text_dim: Color32::from_rgb(148, 155, 168),
            accent: Color32::from_rgb(58, 136, 238),
            accent_soft: Color32::from_rgb(42, 70, 110),
            viewport_bg: Color32::from_rgb(46, 52, 64), // #2e3440 Nord slate
            grid: Color32::from_rgb(59, 65, 76),
            grid_major: Color32::from_rgb(73, 79, 90),
            timeline: Color32::from_rgb(33, 37, 45),
            timeline_item: Color32::from_rgb(52, 58, 70),
            timeline_marker: Color32::from_rgb(58, 136, 238),
            hover: Color32::from_rgb(54, 61, 74),
            field: Color32::from_rgb(30, 34, 41),
            error: Color32::from_rgb(240, 100, 96),
            warning: Color32::from_rgb(236, 170, 70),
            success: Color32::from_rgb(96, 204, 116),
        }
    }
}

pub fn apply_theme(ctx: &egui::Context) {
    let t = Tokens::dark();
    let mut visuals = egui::Visuals::dark();

    visuals.panel_fill = t.panel;
    visuals.window_fill = t.panel;
    visuals.window_stroke = Stroke::new(1.0_f32, t.border);
    visuals.extreme_bg_color = t.field;
    visuals.faint_bg_color = t.panel_header;
    visuals.override_text_color = Some(t.text);

    visuals.widgets.inactive.weak_bg_fill = t.panel_header;
    visuals.widgets.inactive.bg_fill = t.panel_header;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, t.text);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, t.border);

    visuals.widgets.hovered.weak_bg_fill = t.hover;
    visuals.widgets.hovered.bg_fill = t.hover;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, t.accent);

    visuals.widgets.active.weak_bg_fill = t.accent_soft;
    visuals.widgets.active.bg_fill = t.accent;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, t.accent);

    visuals.selection.bg_fill = t.accent_soft;
    visuals.selection.stroke = Stroke::new(1.0_f32, t.accent);

    visuals.window_corner_radius = CornerRadius::same(6);

    ctx.set_visuals(visuals);

    ctx.style_mut(|s| {
        s.spacing.item_spacing = Vec2::new(6.0, 5.0);
        s.spacing.button_padding = Vec2::new(8.0, 4.0);
    });
}
