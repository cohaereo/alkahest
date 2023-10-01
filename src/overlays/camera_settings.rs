use strum::{EnumCount, VariantNames};
use winit::window::Window;

use crate::icons::{ICON_BUG, ICON_CLIPBOARD};
use crate::map::ExtendedHash;
use crate::map_resources::MapResource;
use crate::resources::Resources;
use crate::FpsCamera;

use super::gui::{GuiResources, OverlayProvider};

pub struct CameraPositionOverlay {
    pub show_map_resources: bool,
    pub show_map_resource_label: bool,
    pub map_resource_filter: [bool; MapResource::COUNT],
    pub map_resource_distance: f32,
}

impl OverlayProvider for CameraPositionOverlay {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &Window,
        resources: &mut Resources,
        _icons: &GuiResources,
    ) {
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
                    for (i, n) in MapResource::VARIANTS.iter().enumerate() {
                        ui.checkbox(
                            &mut self.map_resource_filter[i],
                            format!("{} {}", MapResource::get_icon_by_index(i as u8), n),
                        );
                    }
                });
                ui.checkbox(&mut self.show_map_resource_label, "Show map resource label");
                ui.spacing();

                ui.add(
                    egui::Slider::new(&mut self.map_resource_distance, 25.0..=4000.0)
                        .text("Max Resource Distance"),
                );
            }
        });
    }
}

// cohae: Hate it
pub struct CurrentCubemap(pub Option<String>, pub Option<ExtendedHash>);
