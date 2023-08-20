use anyhow::bail;
use anyhow::Context;
use destiny_pkg::TagHash;

use glam::Vec4;

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

use crate::packages::package_manager;

use super::DeviceContextSwapchain;
use super::RenderData;

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
        dcs: &DeviceContextSwapchain,
        render_data: &RenderData,
    ) -> anyhow::Result<()> {
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
                        if let Some(mat) = render_data.materials.get(&mat_hash.0) {
                            if mat.unk8 != 1 {
                                continue;
                            }

                            mat.bind(dcs, render_data)?;
                        }
                    } else {
                        bail!("Could not bind material");
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
                        EPrimitiveType::Triangles => D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
                        EPrimitiveType::TriangleStrip => D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP,
                    });

                    dcs.context.DrawIndexed(p.index_count, p.index_start, 0);
                }
            }
        }

        Ok(())
    }
}
