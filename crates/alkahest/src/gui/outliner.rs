use alkahest_renderer::{
    camera::Camera,
    ecs::{
        common::{Hidden, Icon, Label, Mutable},
        hierarchy::{Children, Parent},
        resources::SelectedEntity,
        tags::{EntityTag, Tags},
        transform::Transform,
        Scene,
    },
    resources::Resources,
    util::color::ColorExt,
};
use egui::{collapsing_header::CollapsingState, Color32, RichText};
use hecs::{Entity, EntityRef};
use itertools::Itertools;
use rustc_hash::FxHashMap;
use strum::IntoEnumIterator;
use winit::window::Window;

use crate::{
    gui::{
        chip::EcsTagsExt,
        context::{GuiCtx, GuiView, ViewResult},
        icons::{ICON_DELETE, ICON_EYE_OFF, ICON_HELP_CIRCLE},
    },
    maplist::{Map, MapList},
    util::text::{alk_color_to_egui, prettify_distance},
};

pub struct OutlinerPanel {
    sort_by_distance: bool,

    filters: FxHashMap<EntityTag, bool>,
}

impl Default for OutlinerPanel {
    fn default() -> Self {
        Self {
            sort_by_distance: false,
            filters: EntityTag::iter()
                .map(|tag| (tag, false))
                .collect::<FxHashMap<_, _>>(),
        }
    }
}

impl GuiView for OutlinerPanel {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &Window,
        resources: &Resources,
        _gui: &GuiCtx<'_>,
    ) -> Option<ViewResult> {
        let mut maps = resources.get_mut::<MapList>();
        if let Some(map) = maps.current_map_mut() {
            let scene = &mut map.scene;

            let camera = resources.get::<Camera>();

            let enabled_filters = self.filters.iter().filter(|(_, v)| **v).count();
            let mut entities = scene
                .query::<(Option<&Transform>, Option<&Tags>)>()
                .without::<&Parent>()
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
                        (transform.translation - camera.position()).length()
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

            // let mut selected_entity = resources.get_mut::<SelectedEntity>();
            // let mut delete_entity = None;

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
                                        .background_color(alk_color_to_egui(tag.color()))
                                        .color(alk_color_to_egui(
                                            tag.color().text_color_for_background(),
                                        )),
                                );
                            }
                        });
                    });
                });

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(
                        ui,
                        // ui.spacing().interact_size.y,
                        // entities.len(),
                        |ui| {
                            for &(ent, _distance) in &entities {
                                self.entity_entry(ui, ent, map, resources);
                            }
                        },
                    );
            });
        }

        None
    }
}

impl OutlinerPanel {
    fn entity_entry(
        &mut self,
        ui: &mut egui::Ui,
        ent: Entity,
        map: &mut Map,
        resources: &Resources,
    ) {
        let e = map.scene.entity(ent).unwrap();

        let children = e.get::<&Children>().as_deref().cloned();

        if let Some(children) = children {
            CollapsingState::load_with_default_open(
                ui.ctx(),
                egui::Id::new(format!("outliner_entity_{ent:?}",)),
                false,
            )
            .show_header(ui, |ui| {
                self.draw_entity_entry(ui, resources, e, &mut map.command_buffer)
            })
            .body_unindented(|ui| {
                ui.style_mut().spacing.indent = 16.0 * 2.;
                ui.indent("outliner_entity_indent", |ui| {
                    for child in children.iter() {
                        self.entity_entry(ui, *child, map, resources);
                    }
                });
            });
        } else {
            self.draw_entity_entry(ui, resources, e, &mut map.command_buffer);
        }
    }

    fn draw_entity_entry(
        &self,
        ui: &mut egui::Ui,
        resources: &Resources,
        e: EntityRef<'_>,
        cmd: &mut hecs::CommandBuffer,
    ) {
        let distance = if let Some(transform) = e.get::<&Transform>() {
            (transform.translation - resources.get::<Camera>().position()).length()
        } else {
            f32::INFINITY
        };

        let postfix = if self.sort_by_distance {
            format!(" ({})", prettify_distance(distance))
        } else {
            "".to_string()
        };

        let visible = !e.has::<Hidden>();
        let prefix_vis = if visible {
            "".to_string()
        } else {
            format!("{} ", ICON_EYE_OFF)
        };

        let label = if let Some(label) = e.get::<&Label>() {
            format!("{label} (id {})", e.entity().id())
        } else {
            format!("Entity {}", e.entity().id())
        };
        let icon = if let Some(icon) = e.get::<&Icon>() {
            icon.0
        } else {
            ICON_HELP_CIRCLE
        };

        ui.horizontal(|ui| {
            let response = ui.selectable_label(
                Some(e.entity()) == resources.get::<SelectedEntity>().selected(),
                RichText::new(format!(
                    "{prefix_vis}{icon} {label}{postfix}" // "{} {}{postfix}",
                                                          // resolve_entity_icon(e).unwrap_or(ICON_CHESS_PAWN),
                                                          // resolve_entity_name(e, true)
                ))
                .color(if visible {
                    Color32::WHITE
                } else {
                    Color32::GRAY
                }),
            );

            response.context_menu(|ui| {
                ui.add_enabled_ui(e.has::<Mutable>(), |ui| {
                    // Delete button
                    if ui.button(format!("{} Delete", ICON_DELETE)).clicked() {
                        resources.get_mut::<SelectedEntity>().deselect();
                        cmd.despawn(e.entity());
                    }
                });
            });

            if response.clicked() {
                resources.get_mut::<SelectedEntity>().select(e.entity());
            }

            if let Some(tags) = e.get::<&Tags>() {
                tags.ui_chips(ui);
            }
        });
    }
}
