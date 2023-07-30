use crate::dxbc::DxbcInputSignature;
use crate::dxgi::DxgiFormat;
use crate::entity::{
    decode_vertices, decode_vertices2, DecodedVertex, DecodedVertexBuffer, ELodCategory,
    EPrimitiveType, IndexBufferHeader, VertexBufferHeader,
};
use crate::statics::{Unk80807194, Unk8080719a, Unk8080719b, Unk808071a7};
use crate::types::{Vector2, Vector3, Vector4};
use crate::vertex_layout::InputElement;
use crate::{material, vertex_layout};
use anyhow::{ensure, Context};
use destiny_pkg::PackageManager;
use glam::{Mat4, Quat, Vec2, Vec3, Vec3A, Vec4};
use itertools::Itertools;
use nohash_hasher::IntMap;
use std::io::Read;
use std::mem::transmute;
use std::ptr;
use tracing::{error, info, warn};
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_R16_UINT, DXGI_FORMAT_R32_UINT,
};

pub struct StaticModelBuffer {
    combined_vertex_buffer: ID3D11Buffer,
    combined_vertex_stride: u32,

    // vertex_buffer: ID3D11Buffer,
    // vertex_stride: u32,
    //
    // vertex2_buffer: Option<ID3D11Buffer>,
    // vertex2_stride: Option<u32>,
    index_buffer: ID3D11Buffer,
    index_format: DXGI_FORMAT,
    index_count: usize,
}

pub struct LoadedTexture {
    pub handle: ID3D11Texture2D,
    pub view: ID3D11ShaderResourceView,
    pub format: DxgiFormat,
}

pub struct StaticModel {
    buffers: Vec<StaticModelBuffer>,
    parts: Vec<Unk8080719a>,
    mesh_groups: Vec<Unk8080719b>,

    model: Unk808071a7,
}

impl StaticModel {
    /// Returns instance scope compatible texcoord (X + YZ)
    pub fn texcoord_transform(&self) -> Vec3 {
        Vec3::new(
            self.model.texture_coordinate_scale.x,
            self.model.texture_coordinate_offset.x,
            self.model.texture_coordinate_offset.y,
        )
    }

    pub fn mesh_transform(&self) -> Mat4 {
        Mat4::from_cols(
            [self.model.model_scale, 0.0, 0.0, self.model.model_offset.x].into(),
            [0.0, self.model.model_scale, 0.0, self.model.model_offset.y].into(),
            [0.0, 0.0, self.model.model_scale, self.model.model_offset.z].into(),
            [0.0, 0.0, 0.0, 1.0].into(),
        )
    }

    pub fn load(
        model: Unk808071a7,
        device: &ID3D11Device,
        pm: &mut PackageManager,
    ) -> anyhow::Result<StaticModel> {
        let header: Unk80807194 = pm.read_tag_struct(model.unk8).unwrap();

        ensure!(header.unk8.len() == model.materials.len());

        let mut buffers = vec![];
        for (index_buffer, vertex_buffer_hash, vertex2_buffer_hash, u3) in header.buffers.iter() {
            let vertex_header: VertexBufferHeader =
                pm.read_tag_struct(*vertex_buffer_hash).unwrap();

            if vertex_header.stride == 24 || vertex_header.stride == 48 {
                warn!("Support for 32-bit floats in vertex buffers are disabled");
                continue;
            }

            let t = pm
                .get_entry_by_tag(*vertex_buffer_hash)
                .unwrap()
                .reference
                .into();

            let vertex_data = pm.read_tag(t).unwrap();

            let mut vertex2_stride = None;
            let mut vertex2_data = None;
            if vertex2_buffer_hash.is_valid() {
                let vertex2_header: VertexBufferHeader =
                    pm.read_tag_struct(*vertex2_buffer_hash).unwrap();
                let t = pm
                    .get_entry_by_tag(*vertex2_buffer_hash)
                    .unwrap()
                    .reference
                    .into();

                vertex2_stride = Some(vertex2_header.stride as u32);
                vertex2_data = Some(pm.read_tag(t).unwrap());
            }

            let index_header: IndexBufferHeader = pm.read_tag_struct(*index_buffer).unwrap();
            let t = pm.get_entry_by_tag(*index_buffer).unwrap().reference.into();
            let index_data = pm.read_tag(t).unwrap();

            let index_buffer = unsafe {
                let buffer = device
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
                    .context("Failed to create index buffer")?;

                buffer
            };

            let combined_vertex_data = if let Some(vertex2_data) = vertex2_data {
                vertex_data
                    .chunks_exact(vertex_header.stride as _)
                    .zip(vertex2_data.chunks_exact(vertex2_stride.unwrap() as _))
                    .map(|(v1, v2)| [v1, v2].concat())
                    .flatten()
                    .collect()
            } else {
                vertex_data
            };

            let combined_vertex_buffer = unsafe {
                device
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

            buffers.push(StaticModelBuffer {
                combined_vertex_buffer,
                combined_vertex_stride: (vertex_header.stride as u32
                    + vertex2_stride.unwrap_or_default()),
                index_buffer,
                index_format: if index_header.is_32bit {
                    DXGI_FORMAT_R32_UINT
                } else {
                    DXGI_FORMAT_R16_UINT
                },
                index_count: if index_header.is_32bit {
                    index_data.len() / 4
                } else {
                    index_data.len() / 2
                } as _,
            })
        }

        Ok(StaticModel {
            buffers,
            model,
            parts: header.parts.to_vec(),
            mesh_groups: header.unk8.to_vec(),
        })
    }

    pub fn draw(
        &self,
        device_context: &ID3D11DeviceContext,
        materials: &IntMap<u32, material::Unk808071e8>,
        vshaders: &IntMap<u32, (ID3D11VertexShader, ID3D11InputLayout)>,
        pshaders: &IntMap<u32, ID3D11PixelShader>,
        cbuffers_vs: &IntMap<u32, ID3D11Buffer>,
        cbuffers_ps: &IntMap<u32, ID3D11Buffer>,
        textures: &IntMap<u32, LoadedTexture>,
        cbuffer_default: ID3D11Buffer,
    ) {
        unsafe {
            for (iu, u) in self
                .mesh_groups
                .iter()
                .enumerate()
                .filter(|(_, u)| u.unk2 == 0)
            // for p in self
            //     .parts
            //     .iter()
            //     .filter(|p| p.lod_category.is_highest_detail())
            {
                let p = &self.parts[u.part_index as usize];
                if !p.lod_category.is_highest_detail() {
                    continue;
                }

                if let Some(buffers) = self.buffers.get(p.buffer_index as usize) {
                    if let Some(mat) = self
                        .model
                        .materials
                        .get(iu)
                        .and_then(|m| materials.get(&m.0))
                    {
                        if let Some(cbuffer) =
                            cbuffers_ps.get(&self.model.materials.get(iu).unwrap().0)
                        {
                            device_context.PSSetConstantBuffers(
                                0,
                                Some(&[
                                    Some(cbuffer.clone()),
                                    Some(cbuffer.clone()),
                                    Some(cbuffer.clone()),
                                    Some(cbuffer.clone()),
                                ]),
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
                        if let Some(cbuffer) =
                            cbuffers_vs.get(&self.model.materials.get(iu).unwrap().0)
                        {
                            device_context.VSSetConstantBuffers(
                                0,
                                Some(&[
                                    Some(cbuffer.clone()),
                                    Some(cbuffer.clone()),
                                    Some(cbuffer.clone()),
                                    Some(cbuffer.clone()),
                                ]),
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

                        // TODO(cohae): Might not go that well if it's None
                        if let Some((vs, input_layout)) = vshaders.get(&mat.vertex_shader.0) {
                            device_context.IASetInputLayout(input_layout);
                            device_context.VSSetShader(vs, None);
                        }

                        if let Some(ps) = pshaders.get(&mat.pixel_shader.0) {
                            device_context.PSSetShader(ps, None);
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

                        device_context.VSSetShaderResources(0, Some(vs_textures.as_slice()));

                        let mut ps_textures = vec![None; ps_tex_count];
                        for p in &mat.ps_textures {
                            if let Some(t) = textures.get(&p.texture.0) {
                                ps_textures[p.index as usize] = Some(t.view.clone());
                            }
                        }

                        device_context.PSSetShaderResources(0, Some(ps_textures.as_slice()));
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

                    device_context.DrawIndexedInstanced(p.index_count, 1, p.index_start, 0, 0);
                }
            }
        }
    }
}
