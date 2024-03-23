use egui::{Align2, Color32};

use super::gui::Overlay;
use crate::{
    ecs::{component_panels::show_inspector_panel, resources::SelectedEntity},
    map::MapList,
};

pub struct InspectorOverlay;

impl Overlay for InspectorOverlay {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &winit::window::Window,
        resources: &mut crate::resources::Resources,
        _gui: &mut super::gui::GuiContext<'_>,
    ) -> bool {
        let mut maps = resources.get_mut::<MapList>().unwrap();

        if let Some(map) = maps.current_map_mut() {
            egui::Window::new("Inspector").show(ctx, |ui| {
                if let Some(ent) = resources.get::<SelectedEntity>().unwrap().0 {
                    show_inspector_panel(
                        ui,
                        &mut map.scene,
                        map.hash,
                        &mut map.command_buffer,
                        ent,
                        resources,
                    );
                } else {
                    ui.colored_label(Color32::WHITE, "No entity selected");
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::WHITE, "Select one using");
                        let p = ui.painter_at(ui.cursor());
                        let pos = ui.cursor().min;
                        ui.label("  ");

                        p.text(
                            pos,
                            Align2::LEFT_TOP,
                            "", // RMB button bg
                            egui::FontId::proportional(
                                ui.text_style_height(&egui::TextStyle::Body),
                            ),
                            Color32::from_rgb(0x33, 0x96, 0xda),
                        );

                        p.text(
                            pos,
                            Align2::LEFT_TOP,
                            "", // RMB button foreground
                            egui::FontId::proportional(
                                ui.text_style_height(&egui::TextStyle::Body),
                            ),
                            Color32::WHITE,
                        );
                    });
                }
            });
        }

        true
    }
}
