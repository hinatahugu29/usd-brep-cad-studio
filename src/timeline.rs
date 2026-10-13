use crate::kernel::{BoolOp, EdgeFeature, EdgeTreatment, PrimitiveKind, ShapeDef};
use crate::theme::Tokens;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

pub struct Timeline;

impl Timeline {
    pub fn show(
        ctx: &egui::Context,
        shapes: &mut Vec<ShapeDef>,
        edge_features: &mut Vec<EdgeFeature>,
        selected_id: &mut Option<u64>,
        rollback_index: &mut usize,
        needs_rebuild: &mut bool,
    ) {
        let t = Tokens::dark();
        let height = 48.0;
        let total = shapes.len() + edge_features.len();

        egui::TopBottomPanel::bottom("cad_timeline")
            .exact_height(height)
            .frame(egui::Frame::NONE.fill(t.timeline))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let painter = ui.painter();

                // Top border line
                painter.line_segment([rect.left_top(), rect.right_top()], Stroke::new(1.0_f32, t.border));

                ui.horizontal_centered(|ui| {
                    ui.add_space(8.0);

                    // Transport controls (operate over the combined shape + edge-feature history)
                    let controls = [
                        ("⏮", "Roll back to start", 0),
                        ("◀", "Step back one feature", rollback_index.saturating_sub(1)),
                        ("▶", "Step forward one feature", (*rollback_index + 1).min(total)),
                        ("⏭", "Play to end", total),
                    ];

                    for (icon, tooltip, target) in controls {
                        let btn = egui::Button::new(icon)
                            .min_size(Vec2::new(26.0, 26.0))
                            .fill(t.panel_header);
                        if ui.add(btn).on_hover_text(tooltip).clicked() {
                            *rollback_index = target;
                            *needs_rebuild = true;
                        }
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Feature cards
                    let mut shape_to_remove = None;

                    egui::ScrollArea::horizontal()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for (idx, shape) in shapes.iter().enumerate() {
                                    let is_active = idx < *rollback_index;
                                    let is_selected = *selected_id == Some(shape.id);

                                    let icon = match shape.kind {
                                        PrimitiveKind::Box => "📦",
                                        PrimitiveKind::Cylinder => "⭕",
                                        PrimitiveKind::Sphere => "🌐",
                                        PrimitiveKind::Cone => "🔺",
                                        PrimitiveKind::Torus => "🍩",
                                    };

                                    let op_tag = match shape.op {
                                        BoolOp::Base => "Base",
                                        BoolOp::Union => "+Join",
                                        BoolOp::Cut => "-Cut",
                                        BoolOp::Intersect => "∩Inters",
                                    };

                                    let bg_fill = if !is_active {
                                        Color32::from_rgb(30, 33, 40) // Dimmed if rolled back
                                    } else if is_selected {
                                        t.accent_soft
                                    } else {
                                        t.timeline_item
                                    };

                                    let border_color = if is_selected {
                                        t.accent
                                    } else if is_active {
                                        t.border
                                    } else {
                                        Color32::from_rgb(45, 50, 60)
                                    };

                                    let card_size = Vec2::new(110.0, 32.0);
                                    let (card_rect, resp) = ui.allocate_exact_size(card_size, Sense::click());

                                    // Draw card
                                    let p = ui.painter();
                                    p.rect(card_rect, 4.0, bg_fill, Stroke::new(1.0_f32, border_color), egui::StrokeKind::Inside);

                                    // Content
                                    let text_color = if is_active { t.text } else { t.text_dim };
                                    let icon_pos = card_rect.left_top() + Vec2::new(6.0, 7.0);
                                    p.text(icon_pos, Align2::LEFT_TOP, icon, FontId::proportional(12.0), text_color);

                                    let name_pos = card_rect.left_top() + Vec2::new(26.0, 4.0);
                                    p.text(name_pos, Align2::LEFT_TOP, &shape.name, FontId::proportional(10.5), text_color);

                                    let op_pos = card_rect.left_top() + Vec2::new(26.0, 18.0);
                                    let op_color = match shape.op {
                                        BoolOp::Base => Color32::from_rgb(180, 190, 205),
                                        BoolOp::Union => Color32::from_rgb(100, 200, 130),
                                        BoolOp::Cut => Color32::from_rgb(240, 110, 110),
                                        BoolOp::Intersect => Color32::from_rgb(220, 170, 80),
                                    };
                                    p.text(op_pos, Align2::LEFT_TOP, op_tag, FontId::proportional(9.0), op_color);

                                    // Click handling
                                    if resp.clicked() {
                                        *selected_id = Some(shape.id);
                                    }

                                    // Context menu
                                    resp.context_menu(|ui| {
                                        ui.label(format!("Feature: {}", shape.name));
                                        ui.separator();
                                        if ui.button("Rollback to this feature").clicked() {
                                            *rollback_index = idx + 1;
                                            *needs_rebuild = true;
                                            ui.close_menu();
                                        }
                                        if ui.button("🗑 Delete Feature").clicked() {
                                            shape_to_remove = Some(shape.id);
                                            ui.close_menu();
                                        }
                                    });

                                    // Rollback marker separator between items
                                    if idx + 1 == *rollback_index {
                                        let marker_r = Rect::from_min_size(
                                            Pos2::new(card_rect.right() + 2.0, card_rect.top() - 4.0),
                                            Vec2::new(4.0, 38.0),
                                        );
                                        p.rect_filled(marker_r, 2.0, t.timeline_marker);
                                    }

                                    ui.add_space(6.0);
                                }

                                // Edge Feature cards (Fillet/Chamfer), continuing the
                                // same combined history index after all shape cards.
                                let mut edge_feature_to_remove = None;
                                let mut edge_feature_to_toggle = None;
                                let shape_count = shapes.len();

                                for (j, feat) in edge_features.iter().enumerate() {
                                    let combined_idx = shape_count + j;
                                    let is_active = combined_idx < *rollback_index && !feat.suppressed;

                                    let icon = match feat.treatment {
                                        EdgeTreatment::Fillet => "🧊",
                                        EdgeTreatment::Chamfer => "📐",
                                        EdgeTreatment::None => "•",
                                    };

                                    let bg_fill = if feat.suppressed {
                                        Color32::from_rgb(40, 32, 32) // Suppressed tint
                                    } else if !is_active {
                                        Color32::from_rgb(30, 33, 40)
                                    } else {
                                        t.timeline_item
                                    };
                                    let border_color = if feat.suppressed {
                                        Color32::from_rgb(120, 70, 70)
                                    } else if is_active {
                                        t.border
                                    } else {
                                        Color32::from_rgb(45, 50, 60)
                                    };

                                    let card_size = Vec2::new(110.0, 32.0);
                                    let (card_rect, resp) = ui.allocate_exact_size(card_size, Sense::click());

                                    let p = ui.painter();
                                    p.rect(card_rect, 4.0, bg_fill, Stroke::new(1.0_f32, border_color), egui::StrokeKind::Inside);

                                    let text_color = if is_active { t.text } else { t.text_dim };
                                    let icon_pos = card_rect.left_top() + Vec2::new(6.0, 7.0);
                                    p.text(icon_pos, Align2::LEFT_TOP, icon, FontId::proportional(12.0), text_color);

                                    let name_pos = card_rect.left_top() + Vec2::new(26.0, 4.0);
                                    p.text(name_pos, Align2::LEFT_TOP, &feat.name, FontId::proportional(10.0), text_color);

                                    let amount_pos = card_rect.left_top() + Vec2::new(26.0, 18.0);
                                    let amount_text = if feat.suppressed {
                                        "Suppressed".to_string()
                                    } else {
                                        format!("{:.2} mm", feat.amount)
                                    };
                                    let amount_color = if feat.suppressed { Color32::from_rgb(220, 120, 120) } else { Color32::from_rgb(180, 190, 205) };
                                    p.text(amount_pos, Align2::LEFT_TOP, amount_text, FontId::proportional(9.0), amount_color);

                                    resp.context_menu(|ui| {
                                        ui.label(format!("Feature: {}", feat.name));
                                        ui.separator();
                                        if ui.button("Rollback to this feature").clicked() {
                                            *rollback_index = combined_idx + 1;
                                            *needs_rebuild = true;
                                            ui.close_menu();
                                        }
                                        let suppress_label = if feat.suppressed { "✓ Unsuppress" } else { "🚫 Suppress" };
                                        if ui.button(suppress_label).clicked() {
                                            edge_feature_to_toggle = Some(feat.id);
                                            ui.close_menu();
                                        }
                                        if ui.button("🗑 Delete Feature").clicked() {
                                            edge_feature_to_remove = Some(feat.id);
                                            ui.close_menu();
                                        }
                                    });

                                    if combined_idx + 1 == *rollback_index {
                                        let marker_r = Rect::from_min_size(
                                            Pos2::new(card_rect.right() + 2.0, card_rect.top() - 4.0),
                                            Vec2::new(4.0, 38.0),
                                        );
                                        p.rect_filled(marker_r, 2.0, t.timeline_marker);
                                    }

                                    ui.add_space(6.0);
                                }

                                if let Some(id) = edge_feature_to_toggle {
                                    if let Some(f) = edge_features.iter_mut().find(|f| f.id == id) {
                                        f.suppressed = !f.suppressed;
                                    }
                                    *needs_rebuild = true;
                                }
                                if let Some(id) = edge_feature_to_remove {
                                    edge_features.retain(|f| f.id != id);
                                    *rollback_index = shapes.len() + edge_features.len();
                                    *needs_rebuild = true;
                                }
                            });
                        });

                    if let Some(id) = shape_to_remove {
                        shapes.retain(|s| s.id != id);
                        if *selected_id == Some(id) {
                            *selected_id = shapes.first().map(|s| s.id);
                        }
                        *rollback_index = shapes.len() + edge_features.len();
                        *needs_rebuild = true;
                    }
                });
            });
    }
}
