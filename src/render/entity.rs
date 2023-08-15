use anyhow::Context;
use destiny_pkg::TagHash;

use glam::Vec4;
use nohash_hasher::IntMap;
use tracing::warn;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;

use crate::entity::EPrimitiveType;
use crate::entity::IndexBufferHeader;
use crate::entity::Unk808072c5;
use crate::entity::Unk8080737e;
use crate::entity::Unk808073a5;
use crate::entity::VertexBufferHeader;
use crate::material;
use crate::packages::package_manager;
use crate::types::Vector4;

use super::static_render::LoadedTexture;
use super::ConstantBuffer;
use super::DeviceContextSwapchain;

pub struct EntityModelBuffer {
    combined_vertex_buffer: ID3D11Buffer,
    combined_vertex_stride: u32,

    index_buffer: ID3D11Buffer,
    index_format: DXGI_FORMAT,
}

pub struct EntityRenderer {
    meshes: Vec<(EntityModelBuffer, Vec<Unk8080737e>)>,

    _material_map: Vec<Unk808072c5>,
    materials: Vec<TagHash>,

    model: Unk808073a5,
}

impl EntityRenderer {
    pub fn texcoord_transform(&self) -> Vec4 {
        Vec4::new(
            self.model.texcoord_scale.x,
            self.model.texcoord_scale.y,
            self.model.texcoord_offset.x,
            self.model.texcoord_offset.y,
        )
    }

    pub fn mesh_scale(&self) -> Vec4 {
        [
            self.model.model_scale.x,
            self.model.model_scale.y,
            self.model.model_scale.z,
            self.model.model_scale.w,
        ]
        .into()
    }

    pub fn mesh_offset(&self) -> Vec4 {
        [
            self.model.model_offset.x,
            self.model.model_offset.y,
            self.model.model_offset.z,
            self.model.model_offset.w,
        ]
        .into()
    }

    pub fn load(
        model: Unk808073a5,
        material_map: Vec<Unk808072c5>,
        materials: Vec<TagHash>,
        dcs: &DeviceContextSwapchain,
    ) -> anyhow::Result<Self> {
        let mut meshes = vec![];

        for mesh in &model.meshes {
            let pm = package_manager();
            let vertex_header: VertexBufferHeader =
                pm.read_tag_struct(mesh.position_buffer).unwrap();

            if vertex_header.stride == 24 || vertex_header.stride == 48 {
                warn!("Support for 32-bit floats in vertex buffers are disabled");
                continue;
            }

            let t = pm.get_entry(mesh.position_buffer).unwrap().reference;

            let vertex_data = pm.read_tag(t).unwrap();

            let mut vertex2_stride = None;
            let mut vertex2_data = None;
            if mesh.secondary_vertex_buffer.is_valid() {
                let vertex2_header: VertexBufferHeader =
                    pm.read_tag_struct(mesh.secondary_vertex_buffer).unwrap();
                let t = pm
                    .get_entry(mesh.secondary_vertex_buffer)
                    .unwrap()
                    .reference;

                vertex2_stride = Some(vertex2_header.stride as u32);
                vertex2_data = Some(pm.read_tag(t).unwrap());
            }

            let index_header: IndexBufferHeader = pm.read_tag_struct(mesh.index_buffer).unwrap();
            let t = pm.get_entry(mesh.index_buffer).unwrap().reference;
            let index_data = pm.read_tag(t).unwrap();

            let index_buffer = unsafe {
                dcs.device
                    .CreateBuffer(
                        &D3D11_BUFFER_DESC {
                            ByteWidth: index_data.len() as _,
                            Usage: D3D11_USAGE_IMMUTABLE,
                            BindFlags: D3D11_BIND_INDEX_BUFFER,
                            ..Default::default()
                        },
                        Some(&D3D11_SUBRESOURCE_DATA {
                            pSysMem: index_data.as_ptr() as _,
                            ..Default::default()
                        }),
                    )
                    .context("Failed to create index buffer")?
            };

            let combined_vertex_data = if let Some(vertex2_data) = vertex2_data {
                vertex_data
                    .chunks_exact(vertex_header.stride as _)
                    .zip(vertex2_data.chunks_exact(vertex2_stride.unwrap() as _))
                    .flat_map(|(v1, v2)| [v1, v2].concat())
                    .collect()
            } else {
                vertex_data
            };

            let combined_vertex_buffer = unsafe {
                dcs.device
                    .CreateBuffer(
                        &D3D11_BUFFER_DESC {
                            ByteWidth: combined_vertex_data.len() as _,
                            Usage: D3D11_USAGE_IMMUTABLE,
                            BindFlags: D3D11_BIND_VERTEX_BUFFER,
                            ..Default::default()
                        },
                        Some(&D3D11_SUBRESOURCE_DATA {
                            pSysMem: combined_vertex_data.as_ptr() as _,
                            ..Default::default()
                        }),
                    )
                    .context("Failed to create combined vertex buffer")?
            };

            meshes.push((
                EntityModelBuffer {
                    combined_vertex_buffer,
                    combined_vertex_stride: (vertex_header.stride as u32
                        + vertex2_stride.unwrap_or_default()),
                    index_buffer,
                    index_format: if index_header.is_32bit {
                        DXGI_FORMAT_R32_UINT
                    } else {
                        DXGI_FORMAT_R16_UINT
                    },
                },
                mesh.parts.to_vec(),
            ))
        }

        Ok(Self {
            meshes,
            _material_map: material_map,
            materials,
            model,
        })
    }

    fn get_material(&self, _index: u16) -> Option<TagHash> {
        // let map = self.material_map.iter().find(|&v| {
        //     (v.material_start as isize..v.material_start as isize + v.material_count as isize)
        //         .contains(&(index as isize))
        // })?;

        // None
        self.materials.first().cloned()
        // self.material_map
        //     .get(index as usize)
        //     .map(|m| self.materials.get((m.material_start) as usize))
        //     .flatten()
        //     .cloned()
    }

    pub fn draw(
        &self,
        device_context: &ID3D11DeviceContext,
        materials: &IntMap<u32, material::Unk808071e8>,
        vshaders: &IntMap<u32, (ID3D11VertexShader, Option<ID3D11InputLayout>)>,
        pshaders: &IntMap<u32, ID3D11PixelShader>,
        cbuffers_vs: &IntMap<u32, ConstantBuffer<Vector4>>,
        cbuffers_ps: &IntMap<u32, ConstantBuffer<Vector4>>,
        textures: &IntMap<u32, LoadedTexture>,
        samplers: &IntMap<u32, ID3D11SamplerState>,
        cbuffer_default: ID3D11Buffer,
    ) {
        unsafe {
            for (buffers, parts) in self.meshes.iter()
            // .enumerate()
            // .filter(|(_, u)| u.unk2 == 0)
            {
                for p in parts {
                    if !p.lod_category.is_highest_detail() {
                        continue;
                    }

                    let mat_hash = if p.variant_shader_index == u16::MAX {
                        Some(p.material)
                    } else {
                        self.get_material(p.variant_shader_index)
                    };

                    if let Some(mat_hash) = mat_hash {
                        if let Some(mat) = materials.get(&mat_hash.0) {
                            for (si, s) in mat.vs_samplers.iter().enumerate() {
                                device_context.VSSetSamplers(
                                    1 + si as u32,
                                    Some(&[samplers.get(&s.sampler.0).cloned()]),
                                );
                            }
                            for (si, s) in mat.ps_samplers.iter().enumerate() {
                                device_context.PSSetSamplers(
                                    1 + si as u32,
                                    Some(&[samplers.get(&s.sampler.0).cloned()]),
                                );
                            }

                            if let Some(cbuffer) = cbuffers_ps.get(&mat_hash.0) {
                                device_context.PSSetConstantBuffers(
                                    0,
                                    Some(&[Some(cbuffer.buffer().clone())]),
                                );
                            } else {
                                device_context.PSSetConstantBuffers(
                                    0,
                                    Some(&[
                                        Some(cbuffer_default.clone()),
                                        Some(cbuffer_default.clone()),
                                        Some(cbuffer_default.clone()),
                                        Some(cbuffer_default.clone()),
                                    ]),
                                );
                            }
                            if let Some(cbuffer) = cbuffers_vs.get(&mat_hash.0) {
                                device_context.VSSetConstantBuffers(
                                    0,
                                    Some(&[Some(cbuffer.buffer().clone())]),
                                );
                            } else {
                                device_context.VSSetConstantBuffers(
                                    0,
                                    Some(&[
                                        Some(cbuffer_default.clone()),
                                        Some(cbuffer_default.clone()),
                                        Some(cbuffer_default.clone()),
                                        Some(cbuffer_default.clone()),
                                    ]),
                                );
                            }

                            // TODO(cohae): Handle invalid shaders
                            if let Some((vs, Some(input_layout))) =
                                vshaders.get(&mat.vertex_shader.0)
                            {
                                device_context.IASetInputLayout(input_layout);
                                device_context.VSSetShader(vs, None);
                            } else if let Some((vs, Some(input_layout))) = materials
                                .get(&p.material.0)
                                .and_then(|m| vshaders.get(&m.vertex_shader.0))
                            {
                                device_context.IASetInputLayout(input_layout);
                                device_context.VSSetShader(vs, None);
                                // } else {
                                //     warn!("No VS/IL!");
                            }

                            if let Some(ps) = pshaders.get(&mat.pixel_shader.0) {
                                device_context.PSSetShader(ps, None);
                                // } else {
                                //     warn!("No PS!");
                            }

                            let vs_tex_count =
                                mat.vs_textures
                                    .iter()
                                    .map(|v| v.index + 1)
                                    .max()
                                    .unwrap_or_default() as usize;

                            let ps_tex_count =
                                mat.ps_textures
                                    .iter()
                                    .map(|v| v.index + 1)
                                    .max()
                                    .unwrap_or_default() as usize;

                            let mut vs_textures = vec![None; vs_tex_count];
                            for p in &mat.vs_textures {
                                if let Some(t) = textures.get(&p.texture.0) {
                                    vs_textures[p.index as usize] = Some(t.view.clone());
                                }
                            }

                            if !vs_textures.is_empty() {
                                device_context
                                    .VSSetShaderResources(0, Some(vs_textures.as_slice()));
                            }

                            let mut ps_textures = vec![None; ps_tex_count];
                            for p in &mat.ps_textures {
                                if let Some(t) = textures.get(&p.texture.0) {
                                    ps_textures[p.index as usize] = Some(t.view.clone());
                                }
                            }

                            if !ps_textures.is_empty() {
                                device_context
                                    .PSSetShaderResources(0, Some(ps_textures.as_slice()));
                            }
                        }
                    }

                    device_context.IASetVertexBuffers(
                        0,
                        1,
                        Some([Some(buffers.combined_vertex_buffer.clone())].as_ptr()),
                        Some([buffers.combined_vertex_stride].as_ptr()),
                        Some(&0),
                    );

                    device_context.IASetIndexBuffer(
                        Some(&buffers.index_buffer),
                        buffers.index_format,
                        0,
                    );
                    device_context.IASetPrimitiveTopology(match p.primitive_type {
                        EPrimitiveType::Triangles => D3D10_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
                        EPrimitiveType::TriangleStrip => D3D10_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP,
                    });

                    device_context.DrawIndexed(p.index_count, p.index_start, 0);
                }
            }
        }
    }
}
