use egui::{Color32, RichText, TextureId};

use crate::{
    packages::package_manager,
    render::DeviceContextSwapchain,
    structure::ExtendedHash,
    texture::{Texture, TextureHeader},
};

use super::gui::Overlay;

pub struct TextureViewer {
    tag: ExtendedHash,
    header: TextureHeader,
    texture: Texture,
    texture_egui: TextureId,
}

impl TextureViewer {
    pub fn new(tag: ExtendedHash, dcs: &DeviceContextSwapchain) -> anyhow::Result<Self> {
        let header = match tag {
            ExtendedHash::Hash32(h) => package_manager().read_tag_struct(h)?,
            ExtendedHash::Hash64(h) => package_manager().read_tag64_struct(h)?,
        };

        let texture = Texture::load(dcs, tag)?;
        Ok(Self {
            tag,
            header,
            texture,
            texture_egui: TextureId::default(),
        })
    }
}

impl Overlay for TextureViewer {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &winit::window::Window,
        _resources: &mut crate::resources::Resources,
        gui: super::gui::GuiContext<'_>,
    ) -> bool {
        if self.texture_egui == TextureId::default() {
            self.texture_egui = gui
                .integration
                .textures_mut()
                .allocate_dx(unsafe { std::mem::transmute(self.texture.view.clone()) });
            assert_ne!(self.texture_egui, TextureId::default());
        }

        let mut open = true;
        egui::Window::new(format!("Texture {}", self.tag))
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Frame::default().show(ui, |ui| {
                    ui.image(self.texture_egui, egui::Vec2::splat(ui.available_width()));
                });

                ui.label(format!(
                    "Texture dimensions: {}x{}x{}",
                    self.header.width, self.header.height, self.header.depth
                ));
                ui.label(format!("Array size: {}", self.header.array_size));
                ui.label(format!("Format: {:?}", self.header.format));
                ui.separator();
                ui.label(
                    RichText::new("TODO: Interaction controls for cubemaps and 3D textures")
                        .color(Color32::YELLOW),
                );
            });

        open
    }
}
