use crate::dxgi::DxgiFormat;
use crate::entity::{
    decode_vertices, decode_vertices2, DecodedVertex, DecodedVertexBuffer, ELodCategory,
    EPrimitiveType, IndexBufferHeader, VertexBufferHeader,
};
use crate::material;
use crate::statics::{Unk80807194, Unk8080719a, Unk8080719b, Unk808071a7};
use crate::types::{Vector2, Vector3, Vector4};
use anyhow::{ensure, Context};
use destiny_pkg::PackageManager;
use glam::{Vec2, Vec3, Vec3A, Vec4};
use itertools::Itertools;
use nohash_hasher::IntMap;
use std::mem::transmute;
use std::ptr;
use tracing::{error, info, warn};
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R32_UINT;

pub struct StaticModelBuffer {
    vertex_buffer: ID3D11Buffer,
    index_buffer: ID3D11Buffer,
    index_count: usize,
    // material: material::Unk808071e8,
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
    pub fn load(
        model: Unk808071a7,
        device: &ID3D11Device,
        device_context: &ID3D11DeviceContext,
        pm: &mut PackageManager,
    ) -> anyhow::Result<StaticModel> {
        let header: Unk80807194 = pm.read_tag_struct(model.unk8).unwrap();

        ensure!(header.unk8.len() == model.materials.len());

        // println!(
        //     "{} materials, {} things, {} parts, {} buffers",
        //     model.materials.len(),
        //     header.unk8.len(),
        //     header.parts.len(),
        //     header.buffers.len()
        // );
        // for (p, m) in header.unk8.iter().zip(model.materials.iter()) {
        //     let part = &header.parts[p.unk0 as usize];
        //
        //     println!("\tu {p:x?} / {:?} / {m:?}", part.lod_category)
        // }

        let mut buffers = vec![];
        for (bi, (index_buffer, vertex_buffer_hash, unk_buffer_hash, u3)) in
            header.buffers.iter().enumerate()
        {
            let vertex_header: VertexBufferHeader =
                pm.read_tag_struct(*vertex_buffer_hash).unwrap();

            let t = pm
                .get_entry_by_tag(*vertex_buffer_hash)
                .unwrap()
                .reference
                .into();

            if vertex_header.stride == 1 {
                warn!(
                    "Weird stride ({}), skipping ({:?})",
                    vertex_header.stride, t
                );
                continue;
            }

            let mut buffer = DecodedVertexBuffer::default();

            let vertex_buffer = pm.read_tag(t).unwrap();
            if let Err(e) = decode_vertices(&vertex_header, &vertex_buffer, &mut buffer) {
                error!("Failed to decode second vertex buffer: {e}");
                continue;
            }

            let index_header: IndexBufferHeader = pm.read_tag_struct(*index_buffer).unwrap();
            let t = pm.get_entry_by_tag(*index_buffer).unwrap().reference.into();
            let index_buffer = pm.read_tag(t).unwrap();

            let indices = if index_header.is_32bit {
                let d: &[u32] = bytemuck::cast_slice(&index_buffer);
                d.to_vec()
            } else {
                let d: &[u16] = bytemuck::cast_slice(&index_buffer);
                let d = d.to_vec();
                d.into_iter().map_into().collect()
            };

            if unk_buffer_hash.is_valid() {
                let unk_header: VertexBufferHeader = pm.read_tag_struct(*unk_buffer_hash).unwrap();
                let t = pm
                    .get_entry_by_tag(*unk_buffer_hash)
                    .unwrap()
                    .reference
                    .into();

                let unk_buffer = pm.read_tag(t).unwrap();
                if let Err(e) = decode_vertices2(&unk_header, &unk_buffer, &mut buffer) {
                    error!("Failed to decode second vertex buffer: {e}");
                    continue;
                }
            }

            let mut vertices = vec![];
            for v in buffer.positions {
                vertices.push(DecodedVertex {
                    position: [
                        v.x * model.model_scale + model.model_offset.x,
                        v.y * model.model_scale + model.model_offset.y,
                        v.z * model.model_scale + model.model_offset.z,
                        v.w,
                    ],
                    tex_coord: Default::default(),
                    normal: [0., 0., 1., 0.],
                    tangent: Default::default(),
                    color: [1., 1., 1., 1.],
                });
            }

            for (i, v) in buffer.tex_coords.iter().enumerate() {
                if i >= vertices.len() {
                    // warn!(
                    //     "Too many texture coordinates (got {}, expected {})",
                    //     buffer.tex_coords.len(),
                    //     vertices.len()
                    // );
                    break;
                }
                vertices[i].tex_coord = [
                    v.x * model.texture_coordinate_scale.x + model.texture_coordinate_offset.x,
                    v.y * model.texture_coordinate_scale.y + model.texture_coordinate_offset.y,
                ];
            }

            for (i, v) in buffer.normals.iter().enumerate() {
                vertices[i].normal = [v.x, v.y, v.z, v.w];
            }

            for (i, v) in buffer.tangents.iter().enumerate() {
                vertices[i].tangent = [v.x, v.y, v.z, v.w];
            }

            for (i, v) in buffer.colors.iter().enumerate() {
                vertices[i].color = [v.x, v.y, v.z, v.w];
            }

            assert_eq!(
                std::mem::size_of::<DecodedVertex>(),
                (4 + 2 + 4 + 4 + 4) * 4
            );
            let bytes: &[u8] = bytemuck::cast_slice(&vertices);
            let vertex_buffer = unsafe {
                let buffer = device
                    .CreateBuffer(
                        &D3D11_BUFFER_DESC {
                            ByteWidth: bytes.len() as _,
                            Usage: D3D11_USAGE_IMMUTABLE,
                            BindFlags: D3D11_BIND_VERTEX_BUFFER,
                            ..Default::default()
                        },
                        Some(&D3D11_SUBRESOURCE_DATA {
                            pSysMem: bytes.as_ptr() as _,
                            ..Default::default()
                        }),
                        // None,
                    )
                    .context("Failed to create vertex buffer")?;

                device_context.Unmap(&buffer, 0);

                buffer
            };

            let index_buffer = unsafe {
                let buffer = device
                    .CreateBuffer(
                        &D3D11_BUFFER_DESC {
                            ByteWidth: (indices.len() * std::mem::size_of::<u32>()) as _,
                            Usage: D3D11_USAGE_IMMUTABLE,
                            BindFlags: D3D11_BIND_INDEX_BUFFER,
                            ..Default::default()
                        },
                        Some(&D3D11_SUBRESOURCE_DATA {
                            pSysMem: indices.as_ptr() as _,
                            ..Default::default()
                        }),
                        // None,
                    )
                    .context("Failed to create index buffer")?;

                device_context.Unmap(&buffer, 0);

                buffer
            };

            // if header
            //     .parts
            //     .iter()
            //     .any(|p| p.index_start == 294 && p.index_count == 435 && p.buffer_index == bi as u8)
            // {
            //     let unk_header: VertexBufferHeader = pm.read_tag_struct(*unk_buffer_hash).unwrap();
            //     error!(
            //         "FUCKED FORMAT stride0={} stride2={} vbuf={:?} ubuf={:?}",
            //         vertex_header.stride, unk_header.stride, vertex_buffer_hash, unk_buffer_hash
            //     )
            // }

            buffers.push(StaticModelBuffer {
                vertex_buffer,
                index_buffer,
                index_count: indices.len(),
                // material: u3,
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
        vshaders: &IntMap<u32, ID3D11VertexShader>,
        pshaders: &IntMap<u32, ID3D11PixelShader>,
        cbuffers: &IntMap<u32, ID3D11Buffer>,
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
                            cbuffers.get(&self.model.materials.get(iu).unwrap().0)
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

                        // TODO(cohae): Might not go that well if it's None
                        if let Some(vs) = vshaders.get(&mat.vertex_shader.0) {
                            device_context.VSSetShader(vs, None);
                        }

                        if let Some(ps) = pshaders.get(&mat.pixel_shader.0) {
                            device_context.PSSetShader(ps, None);
                        }

                        // if !mat.ps_textures.is_empty() {
                        //     if let Some((_, le_texture_view)) =
                        //         textures.get(&mat.ps_textures.first().unwrap().texture.0)
                        //     {
                        //         device_context.PSSetShaderResources(
                        //             0,
                        //             Some(&[Some(le_texture_view.clone())]),
                        //         );
                        //     }
                        // }

                        // let mut ps_textures = vec![];
                        // for pst in &mat.ps_textures {
                        //     if let Some(t) = textures.get(&pst.texture.0) {
                        //         ps_textures.push(t)
                        //     }
                        // }

                        // if let Some(t) = ps_textures.iter().find(|l| l.format.is_srgb())
                        // .or(ps_textures.first())
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
                        Some(&Some(buffers.vertex_buffer.clone())),
                        Some(&((4 + 2 + 4 + 4 + 4) * 4)),
                        Some(&0),
                    );
                    device_context.IASetIndexBuffer(
                        Some(&buffers.index_buffer),
                        DXGI_FORMAT_R32_UINT,
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
