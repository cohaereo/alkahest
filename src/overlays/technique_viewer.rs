use egui::{vec2, Image, ImageSource, Sense, TextureId};
use glam::Vec4;
use nohash_hasher::IntMap;

use crate::{
    packages::package_manager,
    render::{
        bytecode::{
            decompiler::TfxBytecodeDecompiler, externs::TfxShaderStage, opcodes::TfxBytecodeOp,
        },
        dcs::DcsShared,
        DeviceContextSwapchain,
    },
    structure::ExtendedHash,
    technique::{STechnique, STechniqueShader},
    texture::{STextureHeader, Texture},
};

use super::{
    gui::{GuiContext, Overlay, ViewerWindows},
    texture_viewer::TextureViewer,
};

pub struct TechniqueViewer {
    _dcs: DcsShared,

    tag: ExtendedHash,
    _header: STechnique,

    shaders: Vec<TechniqueShaderViewer>,
}

impl TechniqueViewer {
    pub fn new(
        tag: ExtendedHash,
        dcs: DcsShared,
        gui: &mut GuiContext<'_>,
    ) -> anyhow::Result<Self> {
        let header: STechnique = match tag {
            ExtendedHash::Hash32(h) => package_manager().read_tag_struct(h)?,
            ExtendedHash::Hash64(h) => package_manager().read_tag64_struct(h)?,
        };

        // let shaders = header
        //     .all_valid_shaders()
        //     .into_iter()
        //     .map(|(stage, shader)| TechniqueShaderViewer::new(stage, shader.clone(), &dcs, gui))
        //     .collect();
        let mut shaders = vec![];
        for shader in header.all_valid_shaders() {
            shaders.push(TechniqueShaderViewer::new(
                shader.0,
                shader.1.clone(),
                &dcs,
                gui,
            ));
        }

        Ok(Self {
            _dcs: dcs,
            tag,
            _header: header,
            shaders,
        })
    }
}

impl Overlay for TechniqueViewer {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        window: &winit::window::Window,
        resources: &mut crate::resources::Resources,
        gui: &mut GuiContext<'_>,
    ) -> bool {
        let mut open = true;
        // open.tex 000091B7DB39C3C0
        egui::Window::new(format!("Technique {}", self.tag))
            .open(&mut open)
            .show(ctx, |ui| {
                for s in &self.shaders {
                    ui.label(format!("Stage: {:?}", s.stage));
                    s.draw(ui, window, resources, gui);

                    ui.separator();
                }
            });

        open
    }

    fn dispose(
        &mut self,
        _ctx: &egui::Context,
        _resources: &mut crate::resources::Resources,
        gui: &mut GuiContext<'_>,
    ) {
        for s in &mut self.shaders {
            s.dispose(gui);
        }
    }
}

pub struct TechniqueShaderViewer {
    stage: TfxShaderStage,
    header: STechniqueShader,

    textures: IntMap<ExtendedHash, (STextureHeader, Texture, TextureId)>,
}

impl TechniqueShaderViewer {
    pub fn new(
        stage: TfxShaderStage,
        header: STechniqueShader,
        dcs: &DeviceContextSwapchain,
        gui: &mut GuiContext<'_>,
    ) -> Self {
        let mut textures = IntMap::default();

        for assignment in &header.textures {
            let header: anyhow::Result<STextureHeader> = match assignment.texture {
                ExtendedHash::Hash32(h) => package_manager().read_tag_struct(h),
                ExtendedHash::Hash64(h) => package_manager().read_tag64_struct(h),
            };
            let Ok(header) = header else {
                continue;
            };

            let Ok(texture) = Texture::load(dcs, assignment.texture) else {
                continue;
            };
            let texture_egui = gui.integration.textures_mut().allocate_dx((
                unsafe { std::mem::transmute(texture.view.clone()) },
                Some(egui::TextureFilter::Linear),
            ));

            textures.insert(assignment.texture, (header, texture, texture_egui));
        }

        Self {
            stage,
            header,
            textures,
        }
    }

    pub fn draw(
        &self,
        ui: &mut egui::Ui,
        _window: &winit::window::Window,
        resources: &mut crate::resources::Resources,
        gui: &mut GuiContext<'_>,
    ) {
        ui.label(format!("{} textures", self.header.textures.len()));
        ui.label(format!("{} samplers", self.header.samplers.len()));
        ui.label(format!("{} bytecode bytes", self.header.bytecode.len()));
        ui.label(format!(
            "{} bytecode constants",
            self.header.bytecode_constants.len()
        ));

        // open.mat 89E2D080
        if ui.button("Decompile bytecode").clicked() {
            if let Ok(opcodes) =
                TfxBytecodeOp::parse_all(&self.header.bytecode, binrw::Endian::Little)
            {
                let constants: &[Vec4] = if self.header.bytecode_constants.is_empty() {
                    &[]
                } else {
                    bytemuck::cast_slice(&self.header.bytecode_constants)
                };

                match TfxBytecodeDecompiler::decompile(opcodes, constants) {
                    Ok(o) => println!("{}", o.pretty_print()),
                    Err(e) => println!("Failed to decompile bytecode: {e}"),
                }
            }
        }
        ui.collapsing(format!("Textures ({})", self.header.textures.len()), |ui| {
            for assignment in &self.header.textures {
                let mut clicked = false;
                ui.allocate_ui(vec2(ui.available_width(), 96.0), |ui| {
                    ui.horizontal(|ui| {
                        if let Some((tex_header, _tex, tex_egui)) =
                            self.textures.get(&assignment.texture)
                        {
                            clicked |= ui
                                .add(
                                    Image::new(ImageSource::Texture(egui::load::SizedTexture {
                                        id: *tex_egui,
                                        size: vec2(96.0, 96.0),
                                    }))
                                    .sense(Sense::click()),
                                )
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked();

                            ui.vertical(|ui| {
                                let texture_dimension = if tex_header.depth > 1 {
                                    if tex_header.array_size > 1 {
                                        "3DArray"
                                    } else {
                                        "3D"
                                    }
                                } else if (tex_header.array_size % 6) == 0 {
                                    if (tex_header.array_size % 6) > 1 {
                                        "CubeArray"
                                    } else {
                                        "Cube"
                                    }
                                } else {
                                    if tex_header.array_size > 1 {
                                        "2DArray"
                                    } else {
                                        "2D"
                                    }
                                };

                                ui.add_space(24.0);
                                ui.label(format!(
                                    "Texture{texture_dimension} {}",
                                    assignment.texture,
                                ));

                                ui.label(format!(
                                    "{}x{}{}{} {:?}",
                                    tex_header.width,
                                    tex_header.height,
                                    if tex_header.depth > 1 {
                                        format!("x{}", tex_header.depth)
                                    } else {
                                        "".to_string()
                                    },
                                    if tex_header.array_size > 1 {
                                        format!(" {} elements", tex_header.array_size)
                                    } else {
                                        "".to_string()
                                    },
                                    tex_header.format,
                                ));

                                ui.label(format!(
                                    "Texture slot {0} : register(t{0})",
                                    assignment.slot
                                ));
                            });
                        } else {
                            ui.label("Texture not loaded");
                        }
                    });
                });

                if let Some(mut viewers) = resources.get_mut::<ViewerWindows>() {
                    if clicked {
                        let dcs = resources.get::<DcsShared>().unwrap();
                        let tag = assignment.texture;
                        // TODO(cohae): Focus window if already open
                        if !viewers.0.contains_key(&tag.to_string()) {
                            match TextureViewer::new(tag, dcs.clone(), gui) {
                                Ok(o) => {
                                    info!("Successfully loaded texture {tag}");
                                    viewers
                                        .0
                                        .entry(tag.to_string())
                                        .or_insert_with(|| Box::new(o));
                                }
                                Err(e) => {
                                    error!("Failed to load texture {tag}: {e}");
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    fn dispose(&mut self, gui: &mut GuiContext<'_>) {
        for (_, _, texture_egui) in self.textures.values() {
            gui.integration.textures_mut().free(*texture_egui);
        }
    }
}
