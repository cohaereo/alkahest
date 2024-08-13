use alkahest_renderer::{
    camera::Camera,
    ecs::{
        common::{Icon, Label, Mutable},
        hierarchy::{Children, Parent},
        resources::SelectedEntity,
        tags::{EntityTag, Tags},
        transform::Transform,
        visibility::{Visibility, VisibilityHelper},
    },
    resources::AppResources,
    util::{color::ColorExt, text::prettify_distance},
};
use bevy_ecs::{entity::Entity, query::Without, system::Commands, world::EntityRef};
use egui::{collapsing_header::CollapsingState, Color32, RichText};
use itertools::Itertools;
use rustc_hash::FxHashMap;
use strum::IntoEnumIterator;
use winit::window::Window;
use alkahest_renderer::ecs::Scene;
use crate::{
    gui::{
        chip::EcsTagsExt,
        context::{GuiCtx, GuiView, ViewResult},
        icons::{ICON_DELETE, ICON_EYE_OFF},
    },
    maplist::{Map, MapList},
    util::text::alk_color_to_egui,
};

pub struct OutlinerPanel {
    sort_by_distance: bool,

    filters: FxHashMap<EntityTag, bool>,

    search: String,
}

impl Default for OutlinerPanel {
    fn default() -> Self {
        Self {
            sort_by_distance: false,
            filters: EntityTag::iter()
                .map(|tag| (tag, false))
                .collect::<FxHashMap<_, _>>(),
            search: "".to_string(),
        }
    }
}

impl GuiView for OutlinerPanel {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &Window,
        resources: &AppResources,
        _gui: &GuiCtx<'_>,
    ) -> Option<ViewResult> {
        let mut maps = resources.get_mut::<MapList>();
        if let Some(map) = maps.current_map_mut() {
            let scene = &mut map.scene;

            let camera = resources.get::<Camera>();

            fn search(entity: Entity, s: &str, sce: &Scene) -> bool {
                let e = sce.entity(entity);
                let label = if let Some(label) = e.get::<Label>() {
                    format!("{label}")
                } else {
                    return false;
                };

                let children = e.get::<Children>();
                if let Some(children) = children {
                    for child in children.iter() {
                        if search(*child, s, sce) {
                            return true;
                        }
                    }
                }

                if !label.to_lowercase().contains(&s) {
                    return false;
                }

                true
            }

            let enabled_filters = self.filters.iter().filter(|(_, v)| **v).count();
            let mut entities = scene
                .query_filtered::<(Entity, Option<&Transform>, Option<&Tags>), Without<Parent>>()
                .iter(scene)
                .filter(|(e, _, tags)| {
                    // Match search string
                    if !self.search.is_empty() {
                        let s = self.search.to_lowercase();
                        if !search(*e, &s, scene){
                            return false;
                        }
                    }

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
                .map(|(e, transform, _tags)| {
                    let distance = if let Some(transform) = transform {
                        (transform.translation - camera.position()).length()
                    } else {
                        f32::INFINITY
                    };

                    (e, distance)
                })
                .collect_vec();

            entities.sort_by_key(|(e, _)| *e);

            if self.sort_by_distance {
                entities.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());
            }

            // let mut selected_entity = resources.get_mut::<SelectedEntity>();
            // let mut delete_entity = None;

            egui::Window::new("Outliner").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Search"));
                });

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
        resources: &AppResources,
    ) {
        let mut commands = map.commands();
        let e = map.scene.entity(ent);

        let children = e.get::<Children>().cloned();

        if let Some(children) = children {
            CollapsingState::load_with_default_open(
                ui.ctx(),
                egui::Id::new(format!("outliner_entity_{ent:?}",)),
                false,
            )
            .show_header(ui, |ui| {
                self.draw_entity_entry(ui, resources, e, &mut commands)
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
            self.draw_entity_entry(ui, resources, e, &mut commands);
        }
    }

    fn draw_entity_entry(
        &self,
        ui: &mut egui::Ui,
        resources: &AppResources,
        e: EntityRef<'_>,
        cmd: &mut Commands,
    ) {
        let distance = if let Some(transform) = e.get::<Transform>() {
            (transform.translation - resources.get::<Camera>().position()).length()
        } else {
            f32::INFINITY
        };

        let postfix = if let Some(children) = e.get::<Children>() {
            format!(" ({})", children.0.len())
        } else {
            "".to_string()
        };

        let postfix = if self.sort_by_distance {
            format!("{postfix} ({})", prettify_distance(distance))
        } else {
            postfix
        };

        let visible = e.get::<Visibility>().is_visible(0);
        let prefix_vis = if visible {
            "".to_string()
        } else {
            format!("{} ", ICON_EYE_OFF)
        };

        let label = if let Some(label) = e.get::<Label>() {
            format!("{label} (id {})", e.id())
        } else {
            format!("Entity {}", e.id())
        };
        let (icon, color) = if let Some(icon) = e.get::<Icon>() {
            (icon.to_string(), icon.color())
        } else {
            (" ".to_string(), Color32::WHITE)
        };

        ui.horizontal(|ui| {
            let response = ui.selectable_label(
                Some(e.id()) == resources.get::<SelectedEntity>().selected(),
                RichText::new(format!(
                    "{prefix_vis}{icon} {label}{postfix}" // "{} {}{postfix}",
                                                          // resolve_entity_icon(e).unwrap_or(ICON_CHESS_PAWN),
                                                          // resolve_entity_name(e, true)
                ))
                .color(if visible {
                    color
                } else {
                    color.gamma_multiply(0.5)
                }),
            );

            response.context_menu(|ui| {
                ui.add_enabled_ui(e.contains::<Mutable>(), |ui| {
                    // Delete button
                    if ui.button(format!("{} Delete", ICON_DELETE)).clicked() {
                        resources.get_mut::<SelectedEntity>().deselect();
                        cmd.entity(e.id()).despawn();
                    }
                });
            });

            if response.clicked() {
                resources.get_mut::<SelectedEntity>().select(e.id());
            }

            if let Some(tags) = e.get::<Tags>() {
                tags.ui_chips(ui);
            }
        });
    }
}
