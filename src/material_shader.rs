use destiny_pkg::TagHash;
use glam::Vec4;
use itertools::Itertools;
use crate::material::{STechnique, SMaterialTextureAssignment};
use crate::packages::package_manager;
use crate::render::bytecode::interpreter::TfxBytecodeInterpreter;
use crate::render::bytecode::opcodes::TfxBytecodeOp;
use crate::render::{ConstantBuffer, RenderData};
use crate::render::renderer::Renderer;
use crate::structure::{TablePointer, ExtendedHash};
use crate::types::Vector4;

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub enum ShaderType {
    Vertex,
    Pixel,
    Compute
}

impl ShaderType {
    fn load_tfx_bytecode(self, material: &STechnique) -> Option<(TfxBytecodeInterpreter, Vec<Vec4>)> {
        let bytecode_table = match self {
            ShaderType::Vertex => &material.vs_bytecode,
            ShaderType::Pixel => &material.ps_bytecode,
            ShaderType::Compute => unimplemented!(),
        };
        let constants_table = match self {
            ShaderType::Vertex => &material.vs_bytecode_constants,
            ShaderType::Pixel => &material.ps_bytecode_constants,
            ShaderType::Compute => unimplemented!()
        };
        let constants: Vec<Vec4> = if constants_table.is_empty() {
            Vec::new()
        } else {
            bytemuck::cast_slice(constants_table).to_vec()
        };
        return match TfxBytecodeOp::parse_all(&bytecode_table, binrw::Endian::Little) {
            Ok(opcodes) => Some((TfxBytecodeInterpreter::new(opcodes), constants)),
            Err(e) => {
                debug!("Failed to parse {} TFX bytecode: {e} (Data={})", self.to_string(), hex::encode(bytecode_table.data()));
                None
            }
        };
    }

    pub fn load_material_scope(self, material: &STechnique) -> Option<Vec<Vec4>> {
        let custom_header_hash = match self {
            ShaderType::Vertex => material.unke4,
            ShaderType::Pixel => material.unk334,
            ShaderType::Compute => unimplemented!()
        };
        if custom_header_hash.is_some() {
            return Some(ShaderType::create_custom_material_scope(custom_header_hash));
        }

        // Everything except Vertex and Pixel is a placeholder
        let float4_table = match self {
            ShaderType::Vertex => &material.unka8,
            ShaderType::Pixel => &material.unk2f8,
            ShaderType::Compute => unimplemented!()
        };
        if float4_table.len() > 1 && float4_table.iter().any(|v| v.x != 0.0 || v.y != 0.0 || v.z != 0.0 || v.w != 0.0) {
            return Some(ShaderType::create_float4_material_scope(float4_table));
        }

        return match self {
            ShaderType::Vertex => Some(ShaderType::create_default_material_scope()),
            ShaderType::Pixel => None,
            ShaderType::Compute => unimplemented!()
        };
    }

    pub fn to_string(self) -> String {
        return match self {
            ShaderType::Vertex => String::from("Vertex Stage"),
            /*ShaderType::Hull => String::from("Hull Stage"),
            ShaderType::Domain => String::from("Domain Stage"),
            ShaderType::Geometry => String::from("Geometry Stage"),*/
            ShaderType::Pixel => String::from("Pixel/Fragment Stage"),
            ShaderType::Compute => String::from("Compute Stage")
        }
    }

    fn create_custom_material_scope(header_hash: TagHash) -> Vec<Vec4> {
        let buffer_header_ref = package_manager().get_entry(header_hash).unwrap().reference;
        let binding = package_manager().read_tag(buffer_header_ref).unwrap();
        let data: &[Vec4] = bytemuck::cast_slice(&binding);
        trace!("Loading {} material scope elements from {buffer_header_ref:?}.", data.len());
        return data.to_vec();
    }

    fn create_float4_material_scope(table: &TablePointer<Vector4>) -> Vec<Vec4> {
        trace!("Loading float4 array material scope with {} elements.", table.len());
        let array: &[Vec4] = &table.iter().map(|v| Vec4::new(v.x, v.y, v.z, v.w)).collect_vec();
        return array.to_vec();
    }

    fn create_default_material_scope() -> Vec<Vec4> {
        trace!("Loading default material scope.");
        return vec!(Vec4::new(1.0, 1.0, 1.0, 1.0));
    }
}

pub struct MaterialShader {
    pub shader_type: ShaderType,
    pub material_scope_data: Option<Vec<Vec4>>,
    pub material_scope_buffer: Option<ConstantBuffer<Vec4>>,
    pub tfx: Option<(TfxBytecodeInterpreter, Vec<Vec4>)>,
    pub shader: TagHash,
    pub samplers: TablePointer<ExtendedHash>
}

impl MaterialShader {

    pub fn create(shader_type: ShaderType, material: &STechnique) -> MaterialShader {
        if shader_type != ShaderType::Pixel && shader_type != ShaderType::Vertex {
            warn!("Trying to load an unimplemented shader: {}!", &shader_type.to_string());
        }

        let material_scope = shader_type.load_material_scope(material);
        let tfx = shader_type.load_tfx_bytecode(material);

        let shader = match shader_type {
            ShaderType::Vertex => material.vertex_shader,
            ShaderType::Pixel => material.pixel_shader,
            ShaderType::Compute => unimplemented!(),
        };

        let samplers = match shader_type {
            ShaderType::Vertex => &material.vs_samplers,
            ShaderType::Pixel => &material.ps_samplers,
            ShaderType::Compute => unimplemented!(),
        };

        return Self {
            shader_type,
            material_scope_data: material_scope,
            material_scope_buffer: None,
            tfx,
            shader,
            samplers: samplers.clone()
        };
    }

    pub fn setup_material_scope_buffer(&mut self, renderer: &Renderer) {
        if let Some(ref data) = &self.material_scope_data.clone() {
            self.material_scope_buffer = Some(ConstantBuffer::create_array_init(
                renderer.dcs.clone(),
                bytemuck::cast_slice(data)).unwrap());
        }
    }

    pub fn bind(&self, renderer: &Renderer, render_data: &RenderData, textures: &TablePointer<SMaterialTextureAssignment>) {
        unsafe {
            let dcs = renderer.dcs.as_ref();

            match self.shader_type {
                ShaderType::Vertex => {
                    if let Some((vs, _, _)) = render_data.vshaders.get(&self.shader) {
                        dcs.context().VSSetShader(vs, None);
                    } else {
                        // TODO: should still be handled, but not here
                        // anyhow::bail!("No vertex shader/input layout bound");
                    }
                },
                ShaderType::Pixel => {
                    if let Some((ps, _)) = render_data.pshaders.get(&self.shader) {
                        dcs.context().PSSetShader(ps, None);
                    } else {
                        // TODO: should still be handled, but not here
                        // anyhow::bail!("No vertex shader/input layout bound");
                    }
                },
                ShaderType::Compute => unimplemented!(),
            }

            for (si, s) in self.samplers.iter().enumerate() {
                match self.shader_type {
                    ShaderType::Vertex => {
                        dcs.context().VSSetSamplers(
                            1 + si as u32,
                            Some(&[render_data.samplers.get(&s.key()).cloned()]),
                        );
                    },
                    ShaderType::Pixel => {
                        dcs.context().PSSetSamplers(
                            1 + si as u32,
                            Some(&[render_data.samplers.get(&s.key()).cloned()]),
                        );
                    },
                    ShaderType::Compute => unimplemented!(),
                }
            }
    
            if let Some(ref cbuffer) = self.material_scope_buffer {
                match self.shader_type {
                    ShaderType::Vertex => dcs.context().VSSetConstantBuffers(0, Some(&[Some(cbuffer.buffer().clone())])),
                    ShaderType::Pixel => dcs.context().PSSetConstantBuffers(0, Some(&[Some(cbuffer.buffer().clone())])),
                    ShaderType::Compute => unimplemented!(),
                }
            } else {
                match self.shader_type {
                    ShaderType::Vertex => dcs.context().VSSetConstantBuffers(0, Some(&[None])),
                    ShaderType::Pixel => dcs.context().PSSetConstantBuffers(0, Some(&[None])),
                    ShaderType::Compute => unimplemented!(),
                }
            }
    
            for p in textures {
                let tex = render_data
                    .textures
                    .get(&p.texture.key())
                    .unwrap_or(&render_data.fallback_texture);

                match self.shader_type {
                    ShaderType::Vertex => dcs.context().VSSetShaderResources(p.index, Some(&[Some(tex.view.clone())])),
                    ShaderType::Pixel => dcs.context().PSSetShaderResources(p.index, Some(&[Some(tex.view.clone())])),
                    ShaderType::Compute => unimplemented!(),
                }
            } 
        }
    }
}