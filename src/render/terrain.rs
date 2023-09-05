use std::rc::Rc;

use crate::entity::{IndexBufferHeader, VertexBufferHeader};
use crate::map::Unk8080714f;

use crate::packages::package_manager;

use anyhow::Context;
use glam::{Mat4, Vec4};

use windows::Win32::Graphics::Direct3D::D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Buffer, D3D11_BIND_INDEX_BUFFER, D3D11_BIND_VERTEX_BUFFER, D3D11_BUFFER_DESC,
    D3D11_SUBRESOURCE_DATA, D3D11_USAGE_IMMUTABLE,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_R16_UINT, DXGI_FORMAT_R32_UINT,
};

use super::drawcall::{DrawCall, ShadingTechnique, SortValue3d, Transparency};
use super::renderer::Renderer;
use super::{ConstantBuffer, DeviceContextSwapchain};

pub struct TerrainRenderer {
    terrain: Unk8080714f,
    group_cbuffers: Vec<ConstantBuffer<Mat4>>,

    combined_vertex_buffer: ID3D11Buffer,
    combined_vertex_stride: u32,

    index_buffer: ID3D11Buffer,
    index_format: DXGI_FORMAT,
}

impl TerrainRenderer {
    pub fn load(
        terrain: Unk8080714f,
        dcs: Rc<DeviceContextSwapchain>,
    ) -> anyhow::Result<TerrainRenderer> {
        let pm = package_manager();
        let vertex_header: VertexBufferHeader = pm.read_tag_struct(terrain.vertex_buffer).unwrap();

        let t = pm.get_entry(terrain.vertex_buffer).unwrap().reference;

        let vertex_data = pm.read_tag(t).unwrap();

        let mut vertex2_stride = None;
        let mut vertex2_data = None;
        if terrain.vertex2_buffer.is_valid() {
            let vertex2_header: VertexBufferHeader =
                pm.read_tag_struct(terrain.vertex2_buffer).unwrap();
            let t = pm.get_entry(terrain.vertex2_buffer).unwrap().reference;

            vertex2_stride = Some(vertex2_header.stride as u32);
            vertex2_data = Some(pm.read_tag(t).unwrap());
        }

        let index_header: IndexBufferHeader = pm.read_tag_struct(terrain.indices).unwrap();
        let t = pm.get_entry(terrain.indices).unwrap().reference;
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

        let mut group_cbuffers = vec![];
        for group in &terrain.mesh_groups {
            let offset = Vec4::new(
                terrain.unk30.x,
                terrain.unk30.y,
                terrain.unk30.z,
                terrain.unk30.w,
            );

            // let texcoord_transform = Vec4::new(4.0, 4.0, 0.0, 0.0);
            let texcoord_transform =
                Vec4::new(group.unk20.x, group.unk20.y, group.unk20.z, group.unk20.w);

            let scope_terrain = Mat4::from_cols(offset, texcoord_transform, Vec4::ZERO, Vec4::ZERO);

            // FIXME: Weird bug where initial data isn't written
            let cb = ConstantBuffer::create(dcs.clone(), None)?;
            cb.write(&scope_terrain)?;
            group_cbuffers.push(cb);
        }

        Ok(TerrainRenderer {
            terrain,
            group_cbuffers,
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

    pub fn draw(&self, renderer: &mut Renderer) -> anyhow::Result<()> {
        for part in self
            .terrain
            .mesh_parts
            .iter()
            .filter(|u| u.detail_level == 0)
        {
            if let Some(_group) = self.terrain.mesh_groups.get(part.group_index as usize) {
                let cb11 = &self.group_cbuffers[part.group_index as usize];

                // TODO(cohae): Dyemaps
                // if let Some(dyemap) = render_data.textures.get(&group.dyemap.0) {
                //     dcs.context
                //         .PSSetShaderResources(14, Some(&[Some(dyemap.view.clone())]));
                // } else {
                //     dcs.context.PSSetShaderResources(14, Some(&[None]));
                // }

                renderer.push_drawcall(
                    SortValue3d::new()
                        // TODO(cohae): calculate depth
                        .with_depth(u32::MIN)
                        .with_material(part.material.0)
                        .with_technique(ShadingTechnique::Deferred)
                        .with_transparency(Transparency::None),
                    DrawCall {
                        vertex_buffer: self.combined_vertex_buffer.clone(),
                        vertex_buffer_stride: self.combined_vertex_stride,
                        index_buffer: self.index_buffer.clone(),
                        index_format: self.index_format,
                        cb11: Some(cb11.buffer().clone()),
                        variant_material: None,
                        index_start: part.index_start,
                        index_count: part.index_count as _,
                        instance_start: None,
                        instance_count: None,
                        primitive_type: D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP,
                    },
                );
            } else {
                panic!("Could not get terrain mesh group {}", part.group_index)
            }
        }

        Ok(())
    }
}
