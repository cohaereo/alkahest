use std::{fs::File, io::Write};

use egui::{vec2, Color32, ComboBox, RichText, Rounding, TextureId};
use glam::Vec4;
use windows::Win32::Graphics::{
    Direct3D::D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP,
    Direct3D11::{ID3D11PixelShader, ID3D11VertexShader, D3D11_VIEWPORT},
};

use crate::{
    dxgi::DxgiFormat,
    packages::package_manager,
    render::{
        bytecode::externs::TfxShaderStage, dcs::DcsShared, drawcall::ShaderStages,
        gbuffer::RenderTarget, shader, ConstantBuffer,
    },
    structure::ExtendedHash,
    texture::{STextureHeader, Texture},
    util::{self, dds, error::ErrorAlert},
};

use super::gui::{GuiContext, Overlay};

#[repr(C)]
pub struct TextureViewerScope {
    pub channel_mask: Vec4,
    pub mip_level: u32,
    pub depth: f32,
}

pub struct TextureViewer {
    dcs: DcsShared,

    tag: ExtendedHash,
    header: STextureHeader,
    texture: Texture,
    texture_egui: TextureId,
    render_target: RenderTarget,

    scope: ConstantBuffer<TextureViewerScope>,
    viewer_vs: ID3D11VertexShader,
    viewer_ps: ID3D11PixelShader,

    channel_r: bool,
    channel_g: bool,
    channel_b: bool,
    channel_a: bool,

    depth: f32,
    selected_mip: usize,
}

impl TextureViewer {
    pub fn new(
        tag: ExtendedHash,
        dcs: DcsShared,
        gui: &mut GuiContext<'_>,
    ) -> anyhow::Result<Self> {
        let header: STextureHeader = match tag {
            ExtendedHash::Hash32(h) => package_manager().read_tag_struct(h)?,
            ExtendedHash::Hash64(h) => package_manager().read_tag64_struct(h)?,
        };

        let vshader_blob = shader::compile_hlsl(
            include_str!("../../assets/shaders/gui/texture_viewer.hlsl"),
            "VShader",
            "vs_5_0",
            "texture_viewer.hlsl",
        )
        .unwrap();
        let pshader_blob = shader::compile_hlsl(
            include_str!("../../assets/shaders/gui/texture_viewer.hlsl"),
            "PShader",
            "ps_5_0",
            "texture_viewer.hlsl",
        )
        .unwrap();

        let (viewer_vs, _) = shader::load_vshader(&dcs, &vshader_blob)?;
        let (viewer_ps, _) = shader::load_pshader(&dcs, &pshader_blob)?;

        let render_target = RenderTarget::create(
            (header.width as u32, header.height as u32),
            DxgiFormat::B8G8R8A8_UNORM,
            dcs.clone(),
        )?;

        let texture = Texture::load(&dcs, tag)?;
        let texture_egui = gui
            .integration
            .textures_mut()
            .allocate_dx(unsafe { std::mem::transmute(render_target.view.clone()) });

        Ok(Self {
            render_target,
            scope: ConstantBuffer::create(dcs.clone(), None)?,
            viewer_vs,
            viewer_ps,
            dcs,
            tag,
            header,
            texture,
            texture_egui,
            channel_r: true,
            channel_g: true,
            channel_b: true,
            channel_a: false,

            depth: 0.0,
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
        _gui: &mut super::gui::GuiContext<'_>,
    ) -> bool {
        // Render the viewport
        unsafe {
            self.scope
                .write(&TextureViewerScope {
                    channel_mask: Vec4::new(
                        self.channel_r as u32 as f32,
                        self.channel_g as u32 as f32,
                        self.channel_b as u32 as f32,
                        self.channel_a as u32 as f32,
                    ),
                    mip_level: self.selected_mip as u32,
                    depth: self.depth,
                })
                .ok();

            self.scope.bind(0, TfxShaderStage::Pixel);
            self.texture.bind(&self.dcs, 0, ShaderStages::PIXEL);

            self.dcs.context().ClearRenderTargetView(
                &self.render_target.render_target,
                [0.0, 0.0, 0.0, 1.0].as_ptr() as _,
            );
            self.dcs.context().OMSetRenderTargets(
                Some(&[Some(self.render_target.render_target.clone())]),
                None,
            );

            self.dcs.context().OMSetDepthStencilState(None, 0);
            self.dcs.context().OMSetBlendState(None, None, 0xFFFFFFFF);

            self.dcs.context().RSSetViewports(Some(&[D3D11_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: self.header.width as f32,
                Height: self.header.height as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }]));

            self.dcs.context().VSSetShader(&self.viewer_vs, None);
            self.dcs.context().PSSetShader(&self.viewer_ps, None);
            self.dcs
                .context()
                .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
            self.dcs.context().Draw(4, 0);
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

                    if self.header.depth > 1 {
                        ui.horizontal(|ui| {
                            ui.label("Depth");
                            ui.add(
                                egui::Slider::new(&mut self.depth, 0.0..=1.0).clamp_to_range(true),
                            );
                        });
                    }

                    if ui.button("Export image").clicked() {
                        let mut dds_data: Vec<u8> = vec![];
                        let (texture, texture_data) = Texture::load_data(self.tag, true).unwrap();

                        dds::dump_to_dds(&mut dds_data, &texture, &texture_data);
                        if ui.input(|i| i.modifiers.shift) {
                            std::fs::create_dir("./textures/").ok();
                            if let Ok(mut f) =
                                File::create(format!("./textures/{}.dds", self.tag)).err_alert()
                            {
                                f.write_all(&dds_data).ok();
                            }
                        } else {
                            util::export::save_dds_dialog(&dds_data, self.tag.to_string());
                        }
                    }

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
                    RichText::new("TODO: Interaction controls for cubemaps").color(Color32::YELLOW),
                );
            });

        open
    }
}
