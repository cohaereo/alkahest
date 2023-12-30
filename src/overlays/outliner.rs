use itertools::Itertools;

use crate::{
    ecs::{resolve_entity_icon, resolve_entity_name, resources::SelectedEntity, tags::Tags},
    icons::ICON_CHESS_PAWN,
    map::MapDataList,
};

use super::gui::Overlay;

pub struct OutlinerOverlay;

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

            let mut entities = scene.iter().map(|e| e.entity()).collect_vec();
            entities.sort_by_key(|e| e.id());

            let mut selected_entity = resources.get_mut::<SelectedEntity>().unwrap();

            egui::Window::new("Outliner").show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show_rows(
                        ui,
                        ui.spacing().interact_size.y,
                        entities.len(),
                        |ui, range| {
                            for &ent in &entities[range] {
                                let e = scene.entity(ent).unwrap();
                                ui.horizontal(|ui| {
                                    let response = ui.selectable_label(
                                        Some(ent) == selected_entity.0,
                                        format!(
                                            "{} {}",
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
