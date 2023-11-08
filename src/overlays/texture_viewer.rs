use egui::{vec2, Color32, ComboBox, RichText, Rounding, TextureId};

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

    channel_r: bool,
    channel_g: bool,
    channel_b: bool,
    channel_a: bool,

    selected_mip: usize,
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
            channel_r: true,
            channel_g: true,
            channel_b: true,
            channel_a: true,

            selected_mip: 0,
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
        // open.tex 000091B7DB39C3C0
        egui::Window::new(format!("Texture {}", self.tag))
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Frame::default().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let rounding_l = Rounding {
                            ne: 0.0,
                            se: 0.0,
                            nw: 2.0,
                            sw: 2.0,
                        };
                        let rounding_m = Rounding::none();
                        let rounding_r = Rounding {
                            nw: 0.0,
                            sw: 0.0,
                            ne: 2.0,
                            se: 2.0,
                        };

                        ui.style_mut().spacing.item_spacing = [0.0; 2].into();

                        ui.style_mut().visuals.widgets.active.rounding = rounding_l;
                        ui.style_mut().visuals.widgets.hovered.rounding = rounding_l;
                        ui.style_mut().visuals.widgets.inactive.rounding = rounding_l;

                        if ui.selectable_label(self.channel_r, "R").clicked() {
                            self.channel_r = !self.channel_r;
                        }

                        ui.style_mut().visuals.widgets.active.rounding = rounding_m;
                        ui.style_mut().visuals.widgets.hovered.rounding = rounding_m;
                        ui.style_mut().visuals.widgets.inactive.rounding = rounding_m;

                        if ui.selectable_label(self.channel_g, "G").clicked() {
                            self.channel_g = !self.channel_g;
                        }
                        if ui.selectable_label(self.channel_b, "B").clicked() {
                            self.channel_b = !self.channel_b;
                        }

                        ui.style_mut().visuals.widgets.active.rounding = rounding_r;
                        ui.style_mut().visuals.widgets.hovered.rounding = rounding_r;
                        ui.style_mut().visuals.widgets.inactive.rounding = rounding_r;

                        if ui.selectable_label(self.channel_a, "A").clicked() {
                            self.channel_a = !self.channel_a;
                        }

                        ui.style_mut().spacing.item_spacing = vec2(8.0, 3.0);

                        ui.add_space(16.0);

                        ComboBox::from_label("Mip")
                            .wrap(false)
                            .width(128.0)
                            .show_index(
                                ui,
                                &mut self.selected_mip,
                                self.header.mip_count as usize,
                                |i| {
                                    format!(
                                        "{i} - {}x{}",
                                        self.header.width as usize >> i,
                                        self.header.height as usize >> i
                                    )
                                },
                            )
                    });

                    let height_ratio = self.header.height as f32 / self.header.width as f32;
                    ui.image(
                        self.texture_egui,
                        egui::Vec2::new(ui.available_width(), ui.available_width() * height_ratio),
                    );
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
