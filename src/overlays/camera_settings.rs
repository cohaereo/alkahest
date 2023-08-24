use strum::{EnumCount, VariantNames};
use winit::window::Window;

use crate::icons::ICON_BUG;
use crate::map_resources::MapResource;
use crate::resources::Resources;
use crate::FpsCamera;

use super::gui::OverlayProvider;

pub struct CameraPositionOverlay {
    pub show_map_resources: bool,
    pub show_map_resource_label: bool,
    pub map_resource_filter: [bool; MapResource::COUNT],
    pub map_resource_distance: f32,

    pub render_lights: bool,
}

impl OverlayProvider for CameraPositionOverlay {
    fn create_overlay(&mut self, ui: &mut imgui::Ui, _window: &Window, resources: &mut Resources) {
        ui.window(format!("{} Debug", ICON_BUG)).build(|| {
            let mut camera = resources.get_mut::<FpsCamera>().unwrap();
            ui.text(format!("X: {}", camera.position.x));
            ui.text(format!("Y: {}", camera.position.y));
            ui.text(format!("Z: {}", camera.position.z));
            ui.text(format!(
                "Cubemap: {}",
                match resources.get::<CurrentCubemap>().and_then(|c| c.0.clone()) {
                    None => "None".to_string(),
                    Some(s) => s,
                }
            ));
            ui.separator();
            ui.slider("Speed Multiplier", 0.01, 10.0, &mut camera.speed_mul);
            ui.checkbox("Render lights", &mut self.render_lights);
            ui.separator();
            ui.checkbox("Show map resources", &mut self.show_map_resources);
            if self.show_map_resources {
                ui.indent();
                ui.group(|| {
                    for (i, n) in MapResource::VARIANTS.iter().enumerate() {
                        ui.checkbox(
                            format!("{} {}", MapResource::get_icon_by_index(i as u8), n),
                            &mut self.map_resource_filter[i],
                        );
                    }
                });
                ui.unindent();
                ui.checkbox("Show map resource label", &mut self.show_map_resource_label);
                ui.spacing();

                ui.slider(
                    "Debug distance",
                    25.0,
                    4000.0,
                    &mut self.map_resource_distance,
                );
            }
        });
    }
}

// cohae: Hate it
pub struct CurrentCubemap(pub Option<String>);
