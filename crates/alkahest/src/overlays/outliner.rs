use egui::RichText;
use itertools::Itertools;
use rustc_hash::FxHashMap;
use strum::IntoEnumIterator;

use super::gui::Overlay;
use crate::{
    camera::FpsCamera,
    ecs::{
        components::{Mutable, Visible},
        resolve_entity_icon, resolve_entity_name,
        resources::SelectedEntity,
        tags::{EntityTag, Tags},
        transform::Transform,
    },
    icons::{ICON_CHESS_PAWN, ICON_DELETE},
    map::MapList,
    util::text::{prettify_distance, text_color_for_background},
};

pub struct OutlinerOverlay {
    sort_by_distance: bool,

    filters: FxHashMap<EntityTag, bool>,
}

impl Default for OutlinerOverlay {
    fn default() -> Self {
        Self {
            sort_by_distance: false,
            filters: EntityTag::iter()
                .map(|tag| (tag, false))
                .collect::<FxHashMap<_, _>>(),
        }
    }
}

impl Overlay for OutlinerOverlay {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &winit::window::Window,
        resources: &mut crate::resources::Resources,
        _gui: &mut super::gui::GuiContext<'_>,
    ) -> bool {
        let mut maps = resources.get_mut::<MapList>().unwrap();
        if let Some(map) = maps.current_map_mut() {
            let scene = &mut map.scene;

            let camera = resources.get::<FpsCamera>().unwrap();

            let enabled_filters = self.filters.iter().filter(|(_, v)| **v).count();
            let mut entities = scene
                .query::<(Option<&Transform>, Option<&Tags>)>()
                .iter()
                .filter(|(_, (_, tags))| {
                    if enabled_filters == 0 {
                        return true;
                    }

                    // Check if the entity has all the tags that are enabled
                    tags.map_or(false, |tags| {
                        self.filters
                            .iter()
                            .filter(|(_, enabled)| **enabled)
                            .all(|(tag, _)| tags.0.contains(tag))
                    })
                })
                .map(|(e, (transform, _tags))| {
                    let distance = if let Some(transform) = transform {
                        (transform.translation - camera.position).length()
                    } else {
                        f32::INFINITY
                    };

                    (e, distance)
                })
                .collect_vec();

            entities.sort_by_key(|(e, _)| e.id());

            if self.sort_by_distance {
                entities.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());
            }

            let mut selected_entity = resources.get_mut::<SelectedEntity>().unwrap();
            let mut delete_entity = None;

            egui::Window::new("Outliner").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.sort_by_distance, "Sort by distance");

                    let filter_count = if enabled_filters > 0 {
                        format!(" ({})", enabled_filters)
                    } else {
                        "".to_string()
                    };
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Max), |ui| {
                        ui.menu_button(format!("Filters{filter_count}"), |ui| {
                            for tag in EntityTag::iter() {
                                let enabled = self.filters.get_mut(&tag).unwrap();
                                ui.toggle_value(
                                    enabled,
                                    RichText::new(tag.to_string())
                                        .background_color(tag.color())
                                        .color(text_color_for_background(tag.color())),
                                );
                            }
                        });
                    });
                });

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show_rows(
                        ui,
                        ui.spacing().interact_size.y,
                        entities.len(),
                        |ui, range| {
                            for &(ent, distance) in &entities[range] {
                                let e = scene.entity(ent).unwrap();
                                ui.horizontal(|ui| {
                                    let postfix = if self.sort_by_distance {
                                        format!(" ({})", prettify_distance(distance))
                                    } else {
                                        "".to_string()
                                    };

                                    let visible = e.get::<&Visible>().map_or(true, |v| v.0);

                                    let response = ui.selectable_label(
                                        Some(ent) == selected_entity.0,
                                        RichText::new(format!(
                                            "{} {}{postfix}",
                                            resolve_entity_icon(e).unwrap_or(ICON_CHESS_PAWN),
                                            resolve_entity_name(e, true)
                                        ))
                                        .color(
                                            if visible {
                                                egui::Color32::WHITE
                                            } else {
                                                egui::Color32::GRAY
                                            },
                                        ),
                                    );

                                    let response = response.context_menu(|ui| {
                                        ui.add_enabled_ui(e.has::<Mutable>(), |ui| {
                                            // Delete button
                                            if ui
                                                .button(format!("{} Delete", ICON_DELETE))
                                                .clicked()
                                            {
                                                selected_entity.0 = None;
                                                delete_entity = Some(ent);
                                            }
                                        });
                                    });

                                    if response.clicked() {
                                        selected_entity.0 = Some(ent);
                                    }

                                    if let Some(tags) = e.get::<&Tags>() {
                                        tags.ui_chips(ui);
                                    }
                                });
                            }
                        },
                    );
            });

            if let Some(delete) = delete_entity {
                scene.despawn(delete).ok();
            }
        }

        true
    }
}
