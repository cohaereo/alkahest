use crate::entity::{IndexBufferHeader, VertexBufferHeader};
use crate::map::Unk8080714f;

use crate::packages::package_manager;

use anyhow::Context;
use glam::{Mat4, Vec4};

use windows::Win32::Graphics::Direct3D::D3D10_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Buffer, ID3D11Device, ID3D11DeviceContext, D3D11_BIND_INDEX_BUFFER, D3D11_BIND_VERTEX_BUFFER,
    D3D11_BUFFER_DESC, D3D11_MAP_WRITE_DISCARD, D3D11_SUBRESOURCE_DATA, D3D11_USAGE_IMMUTABLE,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_R16_UINT, DXGI_FORMAT_R32_UINT,
};

use super::{RenderData};

pub struct TerrainRenderer {
    terrain: Unk8080714f,

    combined_vertex_buffer: ID3D11Buffer,
    combined_vertex_stride: u32,

    index_buffer: ID3D11Buffer,
    index_format: DXGI_FORMAT,
}

impl TerrainRenderer {
    pub fn load(terrain: Unk8080714f, device: &ID3D11Device) -> anyhow::Result<TerrainRenderer> {
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

        Ok(TerrainRenderer {
            terrain,
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

    pub fn draw(
        &self,
        device_context: &ID3D11DeviceContext,
        render_data: &RenderData,
        cb11: &ID3D11Buffer,
    ) {
        unsafe {
            for part in self
                .terrain
                .mesh_parts
                .iter()
                .filter(|u| u.detail_level == 0)
            {
                // TODO(cohae): Generate these one-time per mesh group, writing these every frame, every part is terribly inefficient
                if let Some(group) = self.terrain.mesh_groups.get(part.group_index as usize) {
                    let bmap = device_context
                        .Map(cb11, 0, D3D11_MAP_WRITE_DISCARD, 0)
                        .unwrap();

                    let offset = Vec4::new(
                        self.terrain.unk30.x,
                        self.terrain.unk30.y,
                        self.terrain.unk30.z,
                        self.terrain.unk30.w,
                    );

                    // let texcoord_transform = Vec4::new(4.0, 4.0, 0.0, 0.0);
                    let texcoord_transform =
                        Vec4::new(group.unk20.x, group.unk20.y, group.unk20.z, group.unk20.w);

                    // * Not scope_instance, same buffer index but not the same format
                    let scope_terrain =
                        Mat4::from_cols(offset, texcoord_transform, Vec4::ZERO, Vec4::ZERO);

                    bmap.pData.copy_from_nonoverlapping(
                        &scope_terrain as *const Mat4 as _,
                        std::mem::size_of::<Mat4>(),
                    );

                    device_context.Unmap(cb11, 0);
                    device_context.VSSetConstantBuffers(11, Some(&[Some(cb11.clone())]));

                    if let Some(dyemap) = render_data.textures.get(&group.dyemap.0) {
                        device_context.PSSetShaderResources(14, Some(&[Some(dyemap.view.clone())]));
                    } else {
                        device_context.PSSetShaderResources(14, Some(&[None]));
                    }
                }

                if let Some(cbuffer) = render_data.cbuffers_ps.get(&part.material.0) {
                    device_context.PSSetConstantBuffers(0, Some(&[Some(cbuffer.buffer().clone())]));
                } else {
                    device_context.PSSetConstantBuffers(0, Some(&[None]));
                }

                let mat = render_data.materials.get(&part.material.0).unwrap();
                for (si, s) in mat.vs_samplers.iter().enumerate() {
                    device_context.VSSetSamplers(
                        1 + si as u32,
                        Some(&[render_data.samplers.get(&s.sampler.0).cloned()]),
                    );
                }
                for (si, s) in mat.ps_samplers.iter().enumerate() {
                    device_context.PSSetSamplers(
                        1 + si as u32,
                        Some(&[render_data.samplers.get(&s.sampler.0).cloned()]),
                    );
                }

                if let Some(cbuffer) = render_data.cbuffers_vs.get(&part.material.0) {
                    device_context.VSSetConstantBuffers(0, Some(&[Some(cbuffer.buffer().clone())]));
                } else {
                    device_context.VSSetConstantBuffers(0, Some(&[None]));
                }

                // TODO(cohae): Might not go that well if it's None
                if let Some((vs, Some(input_layout))) =
                    render_data.vshaders.get(&mat.vertex_shader.0)
                {
                    device_context.IASetInputLayout(input_layout);
                    device_context.VSSetShader(vs, None);
                }

                if let Some(ps) = render_data.pshaders.get(&mat.pixel_shader.0) {
                    device_context.PSSetShader(ps, None);
                }

                let vs_tex_count = mat
                    .vs_textures
                    .iter()
                    .map(|v| v.index + 1)
                    .max()
                    .unwrap_or_default() as usize;

                let ps_tex_count = mat
                    .ps_textures
                    .iter()
                    .map(|v| v.index + 1)
                    .max()
                    .unwrap_or_default() as usize;

                let mut vs_textures = vec![None; vs_tex_count];
                for p in &mat.vs_textures {
                    if let Some(t) = render_data.textures.get(&p.texture.0) {
                        vs_textures[p.index as usize] = Some(t.view.clone());
                    }
                }

                if !vs_textures.is_empty() {
                    device_context.VSSetShaderResources(0, Some(vs_textures.as_slice()));
                }

                let mut ps_textures = vec![None; ps_tex_count];
                for p in &mat.ps_textures {
                    if let Some(t) = render_data.textures.get(&p.texture.0) {
                        ps_textures[p.index as usize] = Some(t.view.clone());
                    }
                }

                if !ps_textures.is_empty() {
                    device_context.PSSetShaderResources(0, Some(ps_textures.as_slice()));
                }

                device_context.IASetVertexBuffers(
                    0,
                    1,
                    Some([Some(self.combined_vertex_buffer.clone())].as_ptr()),
                    Some([self.combined_vertex_stride].as_ptr()),
                    Some(&0),
                );

                device_context.IASetIndexBuffer(Some(&self.index_buffer), self.index_format, 0);
                device_context.IASetPrimitiveTopology(D3D10_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);

                device_context.DrawIndexed(part.index_count as _, part.index_start, 0);
            }
        }
    }
}
