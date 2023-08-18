use crate::entity::{EPrimitiveType, IndexBufferHeader, VertexBufferHeader};
use crate::statics::{Unk80807194, Unk8080719a, Unk8080719b, Unk808071a7};

use anyhow::{ensure, Context};
use glam::{Mat4, Vec3};

use crate::packages::package_manager;

use tracing::warn;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_R16_UINT, DXGI_FORMAT_R32_UINT,
};

use super::{DeviceContextSwapchain, RenderData};

pub struct StaticModelBuffer {
    combined_vertex_buffer: ID3D11Buffer,
    combined_vertex_stride: u32,

    index_buffer: ID3D11Buffer,
    index_format: DXGI_FORMAT,
}

pub struct StaticModel {
    buffers: Vec<StaticModelBuffer>,
    parts: Vec<Unk8080719a>,
    mesh_groups: Vec<Unk8080719b>,

    model: Unk808071a7,
}

impl StaticModel {
    /// Returns instance scope compatible texcoord transformation (X + YZ)
    pub fn texcoord_transform(&self) -> Vec3 {
        Vec3::new(
            self.model.texture_coordinate_scale.x,
            self.model.texture_coordinate_offset.x,
            self.model.texture_coordinate_offset.y,
        )
    }

    // TODO(cohae): Use more conventional methods + transpose
    pub fn mesh_transform(&self) -> Mat4 {
        Mat4::from_cols(
            [self.model.model_scale, 0.0, 0.0, self.model.model_offset.x].into(),
            [0.0, self.model.model_scale, 0.0, self.model.model_offset.y].into(),
            [0.0, 0.0, self.model.model_scale, self.model.model_offset.z].into(),
            [0.0, 0.0, 0.0, 1.0].into(),
        )
    }

    pub fn load(model: Unk808071a7, device: &ID3D11Device) -> anyhow::Result<StaticModel> {
        let pm = package_manager();
        let header: Unk80807194 = pm.read_tag_struct(model.unk8).unwrap();

        ensure!(header.unk8.len() == model.materials.len());

        let mut buffers = vec![];
        for (index_buffer, vertex_buffer_hash, vertex2_buffer_hash, _u3) in header.buffers.iter() {
            let vertex_header: VertexBufferHeader =
                pm.read_tag_struct(*vertex_buffer_hash).unwrap();

            if vertex_header.stride == 24 || vertex_header.stride == 48 {
                warn!("Support for 32-bit floats in vertex buffers are disabled");
                continue;
            }

            let t = pm.get_entry(*vertex_buffer_hash).unwrap().reference;

            let vertex_data = pm.read_tag(t).unwrap();

            let mut vertex2_stride = None;
            let mut vertex2_data = None;
            if vertex2_buffer_hash.is_valid() {
                let vertex2_header: VertexBufferHeader =
                    pm.read_tag_struct(*vertex2_buffer_hash).unwrap();
                let t = pm.get_entry(*vertex2_buffer_hash).unwrap().reference;

                vertex2_stride = Some(vertex2_header.stride as u32);
                vertex2_data = Some(pm.read_tag(t).unwrap());
            }

            let index_header: IndexBufferHeader = pm.read_tag_struct(*index_buffer).unwrap();
            let t = pm.get_entry(*index_buffer).unwrap().reference;
            let index_data = pm.read_tag(t).unwrap();

            let index_buffer = unsafe {
                device
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
        dcs: &DeviceContextSwapchain,
        render_data: &RenderData,
        instance_count: usize,
    ) -> anyhow::Result<()> {
        unsafe {
            for (iu, u) in self
                .mesh_groups
                .iter()
                .enumerate()
                .filter(|(_, u)| u.unk2 == 0)
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
                        .and_then(|m| render_data.materials.get(&m.0))
                    {
                        if mat.unk8 != 1 {
                            continue;
                        }

                        mat.bind(dcs, render_data)?;
                    }

                    dcs.context.IASetVertexBuffers(
                        0,
                        1,
                        Some([Some(buffers.combined_vertex_buffer.clone())].as_ptr()),
                        Some([buffers.combined_vertex_stride].as_ptr()),
                        Some(&0),
                    );

                    dcs.context.IASetIndexBuffer(
                        Some(&buffers.index_buffer),
                        buffers.index_format,
                        0,
                    );
                    dcs.context.IASetPrimitiveTopology(match p.primitive_type {
                        EPrimitiveType::Triangles => D3D10_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
                        EPrimitiveType::TriangleStrip => D3D10_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP,
                    });

                    dcs.context.DrawIndexedInstanced(
                        p.index_count,
                        instance_count as _,
                        p.index_start,
                        0,
                        0,
                    );
                }
            }
        }

        Ok(())
    }
}

// pub fn tmp_filter_material(mat: &material::Unk808071e8) -> bool {
//     if mat.unk8 != 1 {
//         return false;
//     }

//     // 0 ??
//     // 2 double sided?
//     // mat.unkc == 2

//     // 0x0000 ??
//     // 0x0081 seems almost perfect for double sided
//     // mat.unk22 == 0x0081

//     // (mat.unk1c & 0x4000) != 0
//     true

//     // 0x1 ?? (n)
//     // 0x2 ?? (n)
//     // 0x4 dynamics
//     // 0x8 ?? (n)
//     // 0x10 ?? (n)
//     // 0x20 ?? (n)
//     // 0x40
//     // 0x80 ??
//     // 0x100
//     // 0x200 statics
//     // 0x400 some kind of decals??
//     // 0x800
//     // 0x1000
//     // 0x2000 ?? (n)
//     // 0x4000
//     // 0x8000 ?? (n)
//     // (mat.unk18 & 0x8000) != 0
// }
