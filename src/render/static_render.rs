use crate::entity::{EPrimitiveType, IndexBufferHeader, VertexBufferHeader};
use crate::statics::{Unk80807193, Unk80807194, Unk8080719a, Unk8080719b, Unk808071a7};

use anyhow::{ensure, Context};
use destiny_pkg::TagHash;
use glam::{Mat4, Vec3};
use itertools::Itertools;

use crate::packages::package_manager;

use tracing::warn;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_R16_UINT, DXGI_FORMAT_R32_UINT,
};

use super::drawcall::{DrawCall, ShadingTechnique, SortValue3d, Transparency};
use super::renderer::Renderer;

pub struct StaticModelBuffer {
    combined_vertex_buffer: ID3D11Buffer,
    combined_vertex_stride: u32,

    index_buffer: ID3D11Buffer,
    index_format: DXGI_FORMAT,
}

pub struct StaticModel {
    pub buffers: Vec<StaticModelBuffer>,
    pub parts: Vec<Unk8080719a>,
    pub mesh_groups: Vec<Unk8080719b>,

    pub decal_parts: Vec<StaticOverlayModel>,

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

    pub fn load(
        model: Unk808071a7,
        device: &ID3D11Device,
        model_hash: TagHash,
    ) -> anyhow::Result<StaticModel> {
        let pm = package_manager();
        let header: Unk80807194 = pm.read_tag_struct(model.unk8).unwrap();

        ensure!(header.unk8.len() == model.materials.len());

        let mut buffers = vec![];
        for (index_buffer_hash, vertex_buffer_hash, vertex2_buffer_hash, _u3) in
            header.buffers.iter()
        {
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

            let index_header: IndexBufferHeader = pm.read_tag_struct(*index_buffer_hash).unwrap();
            let t = pm.get_entry(*index_buffer_hash).unwrap().reference;
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
            unsafe {
                let name = format!("VB {} (model {})\0", vertex_buffer_hash, model_hash);
                combined_vertex_buffer
                    .SetPrivateData(
                        &WKPDID_D3DDebugObjectName,
                        name.len() as u32 - 1,
                        Some(name.as_ptr() as _),
                    )
                    .expect("Failed to set VS name")
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
            decal_parts: model
                .unk20
                .iter()
                .map(|m| StaticOverlayModel::load(m.clone(), device).unwrap())
                .collect_vec(),
            buffers,
            model,
            parts: header.parts.to_vec(),
            mesh_groups: header.unk8.to_vec(),
        })
    }

    pub fn draw(
        &self,
        renderer: &mut Renderer,
        instance_buffer: ID3D11Buffer,
        instance_count: usize,
        draw_transparent: bool,
    ) -> anyhow::Result<()> {
        if draw_transparent {
            for u in &self.decal_parts {
                u.draw(renderer, instance_buffer.clone(), instance_count);
            }
        } else {
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
                    let primitive_type = match p.primitive_type {
                        EPrimitiveType::Triangles => D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
                        EPrimitiveType::TriangleStrip => D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP,
                    };

                    let material = self.model.materials.get(iu).unwrap();

                    renderer.push_drawcall(
                        SortValue3d::new()
                            // TODO(cohae): calculate depth (need to draw instances separately)
                            .with_depth(u32::MIN)
                            .with_material(material.0)
                            .with_technique(ShadingTechnique::Deferred)
                            .with_transparency(Transparency::Blend),
                        DrawCall {
                            vertex_buffer: buffers.combined_vertex_buffer.clone(),
                            vertex_buffer_stride: buffers.combined_vertex_stride,
                            index_buffer: buffers.index_buffer.clone(),
                            index_format: buffers.index_format,
                            cb11: Some(instance_buffer.clone()),
                            index_start: p.index_start,
                            index_count: p.index_count,
                            instance_start: None,
                            instance_count: Some(instance_count as _),
                            primitive_type,
                        },
                    );
                }
            }
        }

        Ok(())
    }
}

pub struct StaticOverlayModel {
    buffers: StaticModelBuffer,
    index_count: usize,
    model: Unk80807193,
}

impl StaticOverlayModel {
    pub fn load(model: Unk80807193, device: &ID3D11Device) -> anyhow::Result<StaticOverlayModel> {
        let pm = package_manager();
        let vertex_header: VertexBufferHeader = pm.read_tag_struct(model.vertex_buffer).unwrap();

        if vertex_header.stride == 24 || vertex_header.stride == 48 {
            anyhow::bail!("Support for 32-bit floats in vertex buffers are disabled");
        }

        let t = pm.get_entry(model.vertex_buffer).unwrap().reference;

        let vertex_data = pm.read_tag(t).unwrap();

        let mut vertex2_stride = None;
        let mut vertex2_data = None;
        if model.unk10.is_valid() {
            let vertex2_header: VertexBufferHeader = pm.read_tag_struct(model.unk10).unwrap();
            let t = pm.get_entry(model.unk10).unwrap().reference;

            vertex2_stride = Some(vertex2_header.stride as u32);
            vertex2_data = Some(pm.read_tag(t).unwrap());
        }

        let index_header: IndexBufferHeader = pm.read_tag_struct(model.index_buffer).unwrap();
        let t = pm.get_entry(model.index_buffer).unwrap().reference;
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

        let index_count = index_data.len() / if index_header.is_32bit { 4 } else { 2 };

        Ok(Self {
            buffers: StaticModelBuffer {
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
            index_count,
            model,
        })
    }

    pub fn draw(
        &self,
        renderer: &mut Renderer,
        instance_buffer: ID3D11Buffer,
        instance_count: usize,
    ) {
        renderer.push_drawcall(
            SortValue3d::new()
                // TODO(cohae): calculate depth (need to draw instances separately)
                .with_depth(u32::MAX)
                .with_material(self.model.material.0)
                .with_technique(
                    renderer
                        .render_data
                        .material_shading_technique(self.model.material)
                        .unwrap_or(ShadingTechnique::Forward),
                )
                .with_transparency(Transparency::Additive),
            DrawCall {
                vertex_buffer: self.buffers.combined_vertex_buffer.clone(),
                vertex_buffer_stride: self.buffers.combined_vertex_stride,
                index_buffer: self.buffers.index_buffer.clone(),
                index_format: self.buffers.index_format,
                cb11: Some(instance_buffer),
                index_start: 0,
                index_count: self.index_count as _,
                instance_start: None,
                instance_count: Some(instance_count as _),
                primitive_type: D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
            },
        );
    }
}
