use crate::entity::{
    decode_vertices, ELodCategory, EPrimitiveType, IndexBufferHeader, VertexBufferHeader,
};
use crate::statics::{Unk80807194, Unk8080719a, Unk808071a7};
use crate::types::{Vector2, Vector3, Vector4};
use anyhow::Context;
use destiny_pkg::PackageManager;
use itertools::Itertools;
use tracing::warn;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R32_UINT;

pub struct StaticModel {
    vertex_buffer: ID3D11Buffer,
    index_buffer: ID3D11Buffer,

    index_count: usize,
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

        let mut vertex_offset = 0;
        let mut indices: Vec<u32> = vec![];
        let mut vertices: Vec<Vector4> = vec![];
        for (buffer_index, (index_buffer, vertex_buffer, unk_buffer, u3)) in
            header.buffers.iter().enumerate()
        {
            // println!("Extracting buffers i={index_buffer:?} v={vertex_buffer:?} n={unk_buffer:?}");
            // println!("{u3}");
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
            vertices.extend_from_slice(&decode_vertices(&vertex_header, &vertex_buffer));
            // for (v, _) in &verts {
            //     writeln!(
            //         &mut f,
            //         "v {} {} {}",
            //         v.x * mheader.unk6c + mheader.unk50.x,
            //         v.y * mheader.unk6c + mheader.unk50.y,
            //         v.z * mheader.unk6c + mheader.unk50.z,
            //     )
            //     .ok();
            // }

            let index_header: IndexBufferHeader = pm.read_tag_struct(*index_buffer).unwrap();
            let t = pm.get_entry_by_tag(*index_buffer).unwrap().reference.into();
            let index_buffer = pm.read_tag(t).unwrap();

            let raw_indices = if index_header.is_32bit {
                let d: &[u32] = bytemuck::cast_slice(&index_buffer);
                d.to_vec()
            } else {
                let d: &[u16] = bytemuck::cast_slice(&index_buffer);
                let d = d.to_vec();
                d.into_iter().map_into().collect()
            };

            let parts_filtered: Vec<Unk8080719a> = header
                .parts
                .iter()
                .filter(|v| v.buffer_index == buffer_index as u8)
                .cloned()
                .collect();
            let highest_lod = {
                let mut part_lods: Vec<ELodCategory> =
                    parts_filtered.iter().map(|v| v.lod_category).collect();
                part_lods.sort();
                *part_lods.last().unwrap_or(&ELodCategory::Lod_0_0)
            };

            // println!("Using LOD {highest_lod:?}");

            for p in parts_filtered
                .iter()
                .filter(|p| p.lod_category >= highest_lod)
            {
                match p.primitive_type {
                    EPrimitiveType::Triangles => {
                        for i in (0..p.index_count).step_by(3) {
                            let off = (i + p.index_start) as usize;
                            indices.extend_from_slice(&[
                                vertex_offset as u32 + raw_indices[off],
                                vertex_offset as u32 + raw_indices[off + 1],
                                vertex_offset as u32 + raw_indices[off + 2],
                            ]);
                        }
                    }
                    EPrimitiveType::TriangleStrip => {
                        for i in 2..p.index_count {
                            let off = (i + p.index_start) as usize;
                            if (i % 2) == 0 {
                                indices.extend_from_slice(&[
                                    vertex_offset as u32 + raw_indices[off - 2],
                                    vertex_offset as u32 + raw_indices[off - 1],
                                    vertex_offset as u32 + raw_indices[off],
                                ]);
                            } else {
                                indices.extend_from_slice(&[
                                    vertex_offset as u32 + raw_indices[off - 1],
                                    vertex_offset as u32 + raw_indices[off - 2],
                                    vertex_offset as u32 + raw_indices[off],
                                ]);
                            }
                        }
                    }
                }
            }

            vertex_offset = vertices.len();
        }

        let random_uv = (fastrand::f32() % 1.0, fastrand::f32() % 1.0);
        let vertices: Vec<(Vector3, Vector2)> = vertices
            .into_iter()
            .map(|v| {
                (
                    Vector3 {
                        x: v.x * model.model_scale + model.model_offset.x,
                        y: v.y * model.model_scale + model.model_offset.y,
                        z: v.z * model.model_scale + model.model_offset.z,
                    },
                    Vector2 {
                        x: random_uv.0,
                        y: random_uv.1,
                    },
                )
            })
            .collect();

        if vertices.is_empty() || indices.is_empty() {
            anyhow::bail!("Empty vertex/index buffer");
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

        Ok(StaticModel {
            vertex_buffer,
            index_buffer,
            index_count: indices.len(),
            model,
        })
    }

    pub fn draw(&self, device_context: &ID3D11DeviceContext) {
        unsafe {
            device_context.IASetVertexBuffers(
                0,
                1,
                Some(&Some(self.vertex_buffer.clone())),
                Some(&(5 * 4)),
                Some(&0),
            );
            device_context.IASetIndexBuffer(Some(&self.index_buffer), DXGI_FORMAT_R32_UINT, 0);
            device_context.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);

            device_context.DrawIndexed(self.index_count as _, 0, 0);
        }
    }
}
