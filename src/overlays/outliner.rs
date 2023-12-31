use egui::RichText;
use itertools::Itertools;
use nohash_hasher::IntMap;
use strum::IntoEnumIterator;

use crate::{
    camera::FpsCamera,
    ecs::{
        resolve_entity_icon, resolve_entity_name,
        resources::SelectedEntity,
        tags::{EntityTag, Tags},
        transform::Transform,
    },
    icons::ICON_CHESS_PAWN,
    map::MapDataList,
    util::text::{prettify_distance, text_color_for_background},
};

use super::gui::Overlay;

pub struct OutlinerOverlay {
    sort_by_distance: bool,

    filters: IntMap<EntityTag, bool>,
}

impl Default for OutlinerOverlay {
    fn default() -> Self {
        Self {
            sort_by_distance: false,
            filters: EntityTag::iter()
                .map(|tag| (tag, false))
                .collect::<IntMap<_, _>>(),
        }
    }
}

impl Overlay for OutlinerOverlay {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &winit::window::Window,
        resources: &mut crate::resources::Resources,
        _gui: super::gui::GuiContext<'_>,
    ) -> bool {
        let maps = resources.get::<MapDataList>().unwrap();
        if let Some(map) = maps.current_map() {
            let scene = &map.2.scene;

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
                                let mut enabled = self.filters.get_mut(&tag).unwrap();
                                ui.toggle_value(
                                    &mut enabled,
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

                                    let response = ui.selectable_label(
                                        Some(ent) == selected_entity.0,
                                        format!(
                                            "{} {}{postfix}",
                                            resolve_entity_icon(e).unwrap_or(ICON_CHESS_PAWN),
                                            resolve_entity_name(e)
                                        ),
                                    );

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
        }

        true
    }
}
