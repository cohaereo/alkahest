use std::collections::HashMap;
use std::sync::Arc;

use destiny_pkg::TagHash;
use egui::{Context, Ui, Button, vec2, TextureId};
use egui_directx11::texture::TextureAllocator;
use winit::window::Window;
use crate::dxgi::DxgiFormat;
use crate::material::{Technique, SMaterialTextureAssignment};
use crate::material_shader::ShaderType;
use crate::packages::package_manager;
use crate::render::DeviceContextSwapchain;
use crate::resources::Resources;
use crate::texture::{Texture, TextureHeader};

use super::gui::{GuiContext, Overlay};

const MATERIAL_TYPE: u32 = 0x80806DAA;

pub struct MaterialInspector {
    hash_string: String,
    material: Option<Technique>,
    message: String,

    texture_handlers: HashMap<ShaderType, Vec<TextureView>>,
    dcs: Arc<DeviceContextSwapchain>
}

impl MaterialInspector {
    pub fn new(dcs: Arc<DeviceContextSwapchain>) -> MaterialInspector {
        MaterialInspector {
            hash_string: String::new(),
            material: None,
            message: String::new(),
            texture_handlers: HashMap::new(),
            dcs
        }
    }

    pub fn validate_hash(&mut self) -> Result<Technique, String> {
        let tag = u32::from_str_radix(&self.hash_string, 16);

        if let Ok(tag) = tag {
            return match self.load_material(TagHash(u32::from_be(tag))) {
                Some(mat) => Ok(mat),
                None => Err(format!("Unable to find Material for Tag {0}!", self.hash_string))
            };
        }

        return Err("Malformed input tag.".to_string());
    }

    fn load_material(&self, hash: TagHash) -> Option<Technique> {
        match package_manager().get_entry(hash) {
            Some(header) => {
                if header.reference != MATERIAL_TYPE {
                    return None;
                } else {
                    return Some(Technique::load(package_manager().read_tag_struct(hash).unwrap(), hash));
                }
            },
            None => return None
        };
    }

    fn add_category(&self, shader_type: ShaderType, material: &Technique, ui: &mut Ui) {
        if let Some(ref shader) = &material.shaders.get(&shader_type) {
            ui.collapsing(shader_type.to_string(), |ui| {
                let hlsl = &shader.shader;
                ui.horizontal(|ui| {
                    if ui.add_enabled(hlsl != &TagHash::NONE, Button::new("Shader Analyser")).clicked() {
                        //TODO: Open Shader Analyzer. Also, make a Shader Analyzer.
                    } 
    
                    let tfx = &shader.tfx;
                    if ui.add_enabled(tfx.is_some(), Button::new("TFX")).clicked() {
                        //TODO: Open Shader Analyzer. Also, make a Shader Analyzer.
                    }
                });

                if let Some(ref textures) = &self.texture_handlers.get(&shader_type) {
                    ui.separator();
                    ui.collapsing("Textures", |ui| {
                        for texture in textures.iter() {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| { ui.label(egui::RichText::new(format!("{0}", texture.material_slot)).color(egui::Color32::WHITE).size(35.0)) });
                                ui.separator();
                                ui.spacing();
                                ui.image(texture.texture_id, vec2(128.0, 128.0));
                                ui.spacing();
                                ui.separator();
                                ui.spacing();
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new(format!("{0}", texture.hash)).color(egui::Color32::WHITE));
                                    ui.label(egui::RichText::new(format!("{:#?}", texture.texture_type)).color(egui::Color32::LIGHT_GRAY));
                                    if texture.texture_type == TextureType::Texture3D {
                                        ui.label(egui::RichText::new(format!("{0}x{1}x{2}", texture.size[0], texture.size[1], texture.size[2])).color(egui::Color32::LIGHT_GRAY));
                                    } else {
                                        ui.label(egui::RichText::new(format!("{0}x{1}", texture.size[0], texture.size[1])).color(egui::Color32::LIGHT_GRAY));
                                    }

                                    ui.label(egui::RichText::new(format!("{:#?}", texture.format)).color(egui::Color32::DARK_GRAY));
                                });
                            });
                            ui.add_space(5.0);
                        }
                    });
                }
            });
        }
    }

    fn create_texture_handlers(&mut self, shader_type: ShaderType, material: &Technique, gui: &mut TextureAllocator) {
        if let Some(ref textures) = &material.textures.get(&shader_type) {
            if textures.is_empty() {
                return;
            }
            let mut handles: Vec<TextureView> = Vec::new();
            for texture in textures.iter() {
                let view = TextureView::create(texture, gui, self.dcs.as_ref());
                if let Some(view) = view {
                    handles.push(view);
                }
            }
            self.texture_handlers.insert(shader_type, handles);
        }
    }
}

impl Overlay for MaterialInspector {
    fn draw(&mut self, ctx: &Context, _window: &Window, _resources: &mut Resources, gui: GuiContext<'_>) -> bool {
        egui::Window::new("Material Inspector").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let pressed_enter = ui.text_edit_singleline(&mut self.hash_string).lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button("Load").clicked() || pressed_enter {
                    match self.validate_hash() {
                        Ok(mat) => {
                            self.texture_handlers.clear();
                            let allocator = gui.integration.textures_mut();
                            self.create_texture_handlers(ShaderType::Vertex, &mat, allocator);                            
                            self.create_texture_handlers(ShaderType::Pixel, &mat, allocator);
                            self.material = Some(mat);
                        },
                        Err(msg) => {
                            self.message = msg;
                            self.material = None;
                        }
                    }
                }
            });

            if let Some(ref material) = &self.material {
                ui.label(egui::RichText::new(format!("Loaded Material {0}!", self.hash_string)).color(egui::Color32::GREEN));
                ui.separator();
                self.add_category(ShaderType::Vertex, material, ui);
                self.add_category(ShaderType::Pixel, material, ui);
            } else {
                ui.label(egui::RichText::new(&self.message).color(egui::Color32::RED));
            }
        });

        return true;
    }
}

#[derive(Debug, Eq, PartialEq)]
enum TextureType {
    Texture2D,
    Texture3D,
    CubeMap
}

struct TextureView {
    pub texture_id: TextureId,
    pub hash: TagHash,
    pub texture_type: TextureType,
    pub format: DxgiFormat,
    pub size: [u16; 3],
    pub material_slot: u32
}

impl TextureView {

    pub fn create(tex_ref : &SMaterialTextureAssignment, texture_alloc: &mut TextureAllocator, dcs: &DeviceContextSwapchain) -> Option<TextureView> {
        if let Ok(ref texture) = Texture::load(dcs, tex_ref.texture) {
            let texture_id = texture_alloc.allocate_dx(unsafe { std::mem::transmute(texture.view.clone()) });
            let hash = tex_ref.texture.hash32().unwrap();

            let texture_type = match texture.handle {
                crate::texture::TextureHandle::Texture2D(_) => TextureType::Texture2D,
                crate::texture::TextureHandle::TextureCube(_) => TextureType::CubeMap,
                crate::texture::TextureHandle::Texture3D(_) => TextureType::Texture3D,
            };

            let header: TextureHeader = package_manager().read_tag_struct(hash).unwrap();

            let format = header.format;
            let size = [header.width, header.height, header.depth];
            let material_slot = tex_ref.index;

            return Some(Self { texture_id, hash, texture_type, format, size, material_slot });
        }

        return None;
    }
}
