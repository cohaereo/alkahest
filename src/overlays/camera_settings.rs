use egui::{Color32, RichText};

use winit::window::Window;

use crate::icons::{ICON_BUG, ICON_CLIPBOARD};
use crate::map_resources::MapResource;
use crate::resources::Resources;
use crate::structure::ExtendedHash;
use crate::FpsCamera;

use super::gui::Overlay;

pub struct CameraPositionOverlay {
    pub show_map_resources: bool,
    pub show_map_resource_label: bool,

    pub map_resource_only_show_named: bool,
    pub map_resource_show_map: bool,
    pub map_resource_show_activity: bool,

    pub map_resource_label_background: bool,
    pub map_resource_filter: Vec<bool>,
    pub map_resource_distance: f32,
    pub map_resource_distance_limit_enabled: bool,
}

impl Overlay for CameraPositionOverlay {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &Window,
        resources: &mut Resources,
        _gui: super::gui::GuiContext<'_>,
    ) -> bool {
        egui::Window::new(format!("{} Debug", ICON_BUG)).show(ctx, |ui| {
            let mut camera = resources.get_mut::<FpsCamera>().unwrap();
            ui.label(format!("X: {}", camera.position.x));
            ui.label(format!("Y: {}", camera.position.y));
            ui.label(format!("Z: {}", camera.position.z));
            if ui
                .button(const_format::formatcp!(
                    "{} Copy goto command",
                    ICON_CLIPBOARD
                ))
                .clicked()
            {
                ui.output_mut(|o| {
                    o.copied_text = format!(
                        "goto {} {} {}",
                        camera.position.x, camera.position.y, camera.position.z
                    )
                });
            }

            ui.add_space(4.0);
            ui.label(format!(
                "Cubemap: {}",
                match resources.get::<CurrentCubemap>().and_then(|c| c.0.clone()) {
                    None => "None".to_string(),
                    Some(s) => s,
                }
            ));
            ui.separator();
            ui.add(egui::Slider::new(&mut camera.speed_mul, 0.01..=10.0).text("Speed Multiplier"));
            ui.separator();
            ui.checkbox(&mut self.show_map_resources, "Show map resources");
            if self.show_map_resources {
                ui.indent("mapres_indent", |ui| {
                    for i in 0..(MapResource::max_index() + 1) {
                        let name = MapResource::index_to_id(i);
                        let icon = MapResource::debug_icon_from_index(i);
                        let c = MapResource::debug_color_from_index(i);
                        ui.checkbox(
                            &mut self.map_resource_filter[i],
                            RichText::new(format!("{icon} {name}"))
                                .color(Color32::from_rgb(c[0], c[1], c[2])),
                        );
                    }
                });
                ui.checkbox(&mut self.show_map_resource_label, "Show map resource label");
                ui.checkbox(
                    &mut self.map_resource_label_background,
                    "Map resource label background",
                );
                ui.spacing();

                ui.checkbox(
                    &mut self.map_resource_distance_limit_enabled,
                    "Limit resource display distance",
                );
                ui.add_enabled(
                    self.map_resource_distance_limit_enabled,
                    egui::Slider::new(&mut self.map_resource_distance, 25.0..=4000.0)
                        .text("Max Resource Distance"),
                );

                ui.checkbox(&mut self.map_resource_show_map, "Show map resources");

                ui.checkbox(
                    &mut self.map_resource_show_activity,
                    "Show activity resources",
                );

                ui.checkbox(
                    &mut self.map_resource_only_show_named,
                    "Only show named entities",
                );
            }
        });

        true
    }
}

// cohae: Hate it
pub struct CurrentCubemap(pub Option<String>, pub Option<ExtendedHash>);
