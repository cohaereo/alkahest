use itertools::Itertools;

use crate::{
    camera::FpsCamera,
    ecs::{
        resolve_entity_icon, resolve_entity_name, resources::SelectedEntity, tags::Tags,
        transform::Transform,
    },
    icons::ICON_CHESS_PAWN,
    map::MapDataList,
    util::prettify_distance,
};

use super::gui::Overlay;

#[derive(Default)]
pub struct OutlinerOverlay {
    sort_by_distance: bool,
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

            let mut entities = scene
                .query::<Option<&Transform>>()
                .iter()
                .map(|(e, transform)| {
                    let distance = if let Some(transform) = transform {
                        (transform.translation - camera.position).length()
                    } else {
                        0.0
                    };

                    (e, distance)
                })
                .filter(|(_, distance)| *distance != 0.0)
                .collect_vec();

            entities.sort_by_key(|(e, _)| e.id());

            if self.sort_by_distance {
                entities.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());
            }

            let mut selected_entity = resources.get_mut::<SelectedEntity>().unwrap();

            egui::Window::new("Outliner").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.sort_by_distance, "Sort by distance");
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
