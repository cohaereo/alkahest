use crate::entity::{
    decode_vertices, decode_vertices2, ELodCategory, EPrimitiveType, IndexBufferHeader,
    VertexBufferHeader,
};
use crate::material;
use crate::statics::{Unk80807194, Unk8080719a, Unk8080719b, Unk808071a7};
use crate::types::{Vector2, Vector3, Vector4};
use anyhow::{ensure, Context};
use destiny_pkg::PackageManager;
use itertools::Itertools;
use nohash_hasher::IntMap;
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
        for (index_buffer, vertex_buffer, unk_buffer, u3) in &header.buffers {
            let vertex_header: VertexBufferHeader = pm.read_tag_struct(*vertex_buffer).unwrap();

            let t = pm
                .get_entry_by_tag(*vertex_buffer)
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

            let vertex_buffer = pm.read_tag(t).unwrap();
            let vertices = &decode_vertices(&vertex_header, &vertex_buffer);

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

            // let random_uv = (fastrand::f32() % 1.0, fastrand::f32() % 1.0);
            let mut vertices: Vec<(Vector3, Vector2)> = vertices
                .into_iter()
                .map(|v| {
                    (
                        Vector3 {
                            x: v.x * model.model_scale + model.model_offset.x,
                            y: v.y * model.model_scale + model.model_offset.y,
                            z: v.z * model.model_scale + model.model_offset.z,
                        },
                        Vector2 { x: 0.5, y: 0.5 },
                    )
                })
                .collect();

            if unk_buffer.is_valid() {
                let unk_header: VertexBufferHeader = pm.read_tag_struct(*unk_buffer).unwrap();
                let t = pm.get_entry_by_tag(*unk_buffer).unwrap().reference.into();

                let unk_buffer = pm.read_tag(t).unwrap();
                let unk_vertices = &decode_vertices2(&unk_header, &unk_buffer);
                for (i, v) in unk_vertices.iter().enumerate() {
                    vertices[i].1 = Vector2 {
                        x: v.x * model.texture_coordinate_scale.x
                            + model.texture_coordinate_offset.x,
                        y: v.y * model.texture_coordinate_scale.y
                            + model.texture_coordinate_offset.y,
                    };
                }
            }

            let vertex_buffer = unsafe {
                let buffer = device
                    .CreateBuffer(
                        &D3D11_BUFFER_DESC {
                            ByteWidth: (vertices.len() * std::mem::size_of::<[f32; 5]>()) as _,
                            Usage: D3D11_USAGE_IMMUTABLE,
                            BindFlags: D3D11_BIND_VERTEX_BUFFER,
                            ..Default::default()
                        },
                        Some(&D3D11_SUBRESOURCE_DATA {
                            pSysMem: vertices.as_ptr() as _,
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
        textures: &IntMap<u32, (ID3D11Texture2D, ID3D11ShaderResourceView)>,
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

                        for pst in &mat.ps_textures {
                            if let Some((_, le_texture_view)) = textures.get(&pst.texture.0) {
                                device_context.PSSetShaderResources(
                                    pst.index,
                                    Some(&[Some(le_texture_view.clone())]),
                                );
                            }
                        }
                    }

                    device_context.IASetVertexBuffers(
                        0,
                        1,
                        Some(&Some(buffers.vertex_buffer.clone())),
                        Some(&(5 * 4)),
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

                    device_context.DrawIndexed(p.index_count, p.index_start, 0);
                }
            }
        }
    }
}
