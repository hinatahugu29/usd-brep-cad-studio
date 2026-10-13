use crate::theme::Tokens;
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Key, Pos2, RichText, Stroke, Vec2};

pub struct CommandItem {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub category: &'static str,
}

pub const COMMANDS: &[CommandItem] = &[
    CommandItem { id: "box", name: "Create Box", icon: "📦", category: "CREATE" },
    CommandItem { id: "cyl", name: "Create Cylinder", icon: "⭕", category: "CREATE" },
    CommandItem { id: "sphere", name: "Create Sphere", icon: "🌐", category: "CREATE" },
    CommandItem { id: "cone", name: "Create Cone", icon: "🔺", category: "CREATE" },
    CommandItem { id: "torus", name: "Create Torus", icon: "🍩", category: "CREATE" },
    CommandItem { id: "sketch", name: "Create 2D Sketch", icon: "✏", category: "SKETCH" },
    CommandItem { id: "fillet", name: "Fillet Edges", icon: "🧊", category: "MODIFY" },
    CommandItem { id: "chamfer", name: "Chamfer Edges", icon: "📐", category: "MODIFY" },
    CommandItem { id: "params", name: "User Parameters Table", icon: "⚙", category: "MODIFY" },
    CommandItem { id: "section", name: "Section Analysis Slice", icon: "📊", category: "INSPECT" },
    CommandItem { id: "measure", name: "Toggle 3D Measure", icon: "📏", category: "INSPECT" },
    CommandItem { id: "fit", name: "Fit Model to View (F)", icon: "🎯", category: "VIEW" },
    CommandItem { id: "iso", name: "Isometric View", icon: "⌂", category: "VIEW" },
    CommandItem { id: "top", name: "Top View", icon: "🔲", category: "VIEW" },
    CommandItem { id: "front", name: "Front View", icon: "🔲", category: "VIEW" },
    CommandItem { id: "right", name: "Right View", icon: "🔲", category: "VIEW" },
    CommandItem { id: "step", name: "Export STEP File", icon: "📤", category: "EXPORT" },
    CommandItem { id: "glb", name: "Export GLB File", icon: "📤", category: "EXPORT" },
    CommandItem { id: "stl", name: "Export STL File", icon: "📤", category: "EXPORT" },
    CommandItem { id: "clear", name: "Clear Scene", icon: "🗑", category: "EDIT" },
];

pub struct ShortcutBox {
    pub is_open: bool,
    pub pos: Pos2,
    pub query: String,
    pub selected_idx: usize,
}

impl Default for ShortcutBox {
    fn default() -> Self {
        Self {
            is_open: false,
            pos: Pos2::new(300.0, 200.0),
            query: String::new(),
            selected_idx: 0,
        }
    }
}

impl ShortcutBox {
    pub fn open_at(&mut self, pos: Pos2) {
        self.is_open = true;
        self.pos = pos;
        self.query.clear();
        self.selected_idx = 0;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.query.clear();
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<&'static str> {
        if !self.is_open {
            return None;
        }

        let t = Tokens::dark();
        let mut executed_command = None;

        let filtered: Vec<&CommandItem> = COMMANDS
            .iter()
            .filter(|c| {
                if self.query.is_empty() {
                    return true;
                }
                let q = self.query.to_lowercase();
                c.name.to_lowercase().contains(&q)
                    || c.category.to_lowercase().contains(&q)
                    || c.id.to_lowercase().contains(&q)
            })
            .collect();

        // Keyboard navigation
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.close();
            return None;
        }

        if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
            if !filtered.is_empty() {
                self.selected_idx = (self.selected_idx + 1).min(filtered.len() - 1);
            }
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
            if self.selected_idx > 0 {
                self.selected_idx -= 1;
            }
        }
        if ctx.input(|i| i.key_pressed(Key::Enter)) {
            if let Some(item) = filtered.get(self.selected_idx) {
                executed_command = Some(item.id);
                self.close();
                return executed_command;
            }
        }

        // Clamp index
        if !filtered.is_empty() && self.selected_idx >= filtered.len() {
            self.selected_idx = filtered.len() - 1;
        }

        let window_w = 320.0;
        let window_h = 280.0;

        egui::Window::new("Quick Search (S)")
            .id(egui::Id::new("usd_brep_shortcut_box"))
            .fixed_pos(self.pos)
            .fixed_size([window_w, window_h])
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(Color32::from_rgb(30, 34, 42))
                    .stroke(Stroke::new(1.5_f32, t.accent))
                    .corner_radius(CornerRadius::same(8)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔍").size(14.0));
                    let text_edit = egui::TextEdit::singleline(&mut self.query)
                        .hint_text("Type a command... (Esc to close)")
                        .desired_width(window_w - 50.0);
                    let resp = ui.add(text_edit);
                    resp.request_focus();
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(2.0);

                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for (idx, cmd) in filtered.iter().enumerate() {
                            let is_selected = idx == self.selected_idx;
                            let bg_color = if is_selected { t.accent_soft } else { Color32::TRANSPARENT };

                            let frame = egui::Frame::NONE
                                .fill(bg_color)
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(egui::Margin::symmetric(6, 4));

                            frame.show(ui, |ui| {
                                let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), egui::Sense::click());
                                let painter = ui.painter();

                                let icon_pos = rect.left_top() + Vec2::new(2.0, 2.0);
                                painter.text(icon_pos, Align2::LEFT_TOP, cmd.icon, FontId::proportional(12.0), Color32::WHITE);

                                let name_pos = rect.left_top() + Vec2::new(24.0, 3.0);
                                let name_color = if is_selected { Color32::WHITE } else { t.text };
                                painter.text(name_pos, Align2::LEFT_TOP, cmd.name, FontId::proportional(12.0), name_color);

                                let cat_pos = rect.right_top() + Vec2::new(-6.0, 4.0);
                                painter.text(cat_pos, Align2::RIGHT_TOP, cmd.category, FontId::proportional(9.0), t.text_dim);

                                if resp.clicked() {
                                    executed_command = Some(cmd.id);
                                    self.close();
                                }
                            });
                        }
                    });
            });

        executed_command
    }
}
