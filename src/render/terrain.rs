use std::sync::Arc;


use crate::map::Unk8080714f;

use crate::packages::package_manager;


use destiny_pkg::TagHash;
use glam::{Mat4, Vec4};

use windows::Win32::Graphics::Direct3D::D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP;


use super::drawcall::{DrawCall, ShadingTechnique, SortValue3d, Transparency};
use super::renderer::Renderer;
use super::vertex_buffers::load_vertex_buffers;
use super::{ConstantBuffer, DeviceContextSwapchain};

pub struct TerrainRenderer {
    terrain: Unk8080714f,
    group_cbuffers: Vec<ConstantBuffer<Mat4>>,

    vertex_buffer1: TagHash,
    vertex_buffer2: TagHash,
    input_layout: u64,

    index_buffer: TagHash,
}

impl TerrainRenderer {
    pub fn load(
        terrain: Unk8080714f,
        dcs: Arc<DeviceContextSwapchain>,
        renderer: &Renderer,
    ) -> anyhow::Result<TerrainRenderer> {
        let _pm = package_manager();
        // let vertex_header: VertexBufferHeader = pm.read_tag_struct(terrain.vertex_buffer).unwrap();

        // let t = pm.get_entry(terrain.vertex_buffer).unwrap().reference;

        // let vertex_data = pm.read_tag(t).unwrap();

        // let mut vertex2_stride = None;
        // let mut vertex2_data = None;
        // if terrain.vertex_buffer2.is_valid() {
        //     let vertex2_header: VertexBufferHeader =
        //         pm.read_tag_struct(terrain.vertex_buffer2).unwrap();
        //     let t = pm.get_entry(terrain.vertex_buffer2).unwrap().reference;

        //     vertex2_stride = Some(vertex2_header.stride as u32);
        //     vertex2_data = Some(pm.read_tag(t).unwrap());
        // }

        // let combined_vertex_data = if let Some(vertex2_data) = vertex2_data {
        //     vertex_data
        //         .chunks_exact(vertex_header.stride as _)
        //         .zip(vertex2_data.chunks_exact(vertex2_stride.unwrap() as _))
        //         .flat_map(|(v1, v2)| [v1, v2].concat())
        //         .collect()
        // } else {
        //     vertex_data
        // };

        // let combined_vertex_buffer = unsafe {
        //     dcs.device
        //         .CreateBuffer(
        //             &D3D11_BUFFER_DESC {
        //                 ByteWidth: combined_vertex_data.len() as _,
        //                 Usage: D3D11_USAGE_IMMUTABLE,
        //                 BindFlags: D3D11_BIND_VERTEX_BUFFER,
        //                 ..Default::default()
        //             },
        //             Some(&D3D11_SUBRESOURCE_DATA {
        //                 pSysMem: combined_vertex_data.as_ptr() as _,
        //                 ..Default::default()
        //             }),
        //         )
        //         .context("Failed to create combined vertex buffer")?
        // };

        renderer.render_data.load_buffer(terrain.indices);
        renderer.render_data.load_buffer(terrain.vertex_buffer);
        renderer.render_data.load_buffer(terrain.vertex_buffer2);

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

        for p in &terrain.mesh_parts {
            renderer.render_data.load_material(renderer, p.material);
        }

        // Find the first normal material to use for the input layout
        let mut buffer_layout_material = TagHash(u32::MAX);
        for m in terrain.mesh_parts.iter() {
            if let Some(mat) = renderer.render_data.data().materials.get(&m.material) {
                if mat.unk8 == 1 {
                    buffer_layout_material = m.material;
                    break;
                }
            }
        }

        let input_layout = load_vertex_buffers(
            renderer,
            buffer_layout_material,
            &[terrain.vertex_buffer, terrain.vertex_buffer2],
        )?;

        Ok(TerrainRenderer {
            group_cbuffers,
            vertex_buffer1: terrain.vertex_buffer,
            vertex_buffer2: terrain.vertex_buffer2,
            index_buffer: terrain.indices,
            input_layout,
            terrain,
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
                //     dcs.context().PSSetShaderResources(14, Some(&[None]));
                // }

                renderer.push_drawcall(
                    SortValue3d::empty()
                        // TODO(cohae): calculate depth
                        .with_depth(u32::MIN)
                        .with_material(part.material.0)
                        .with_technique(ShadingTechnique::Deferred)
                        .with_transparency(Transparency::None),
                    DrawCall {
                        vertex_buffers: vec![self.vertex_buffer1, self.vertex_buffer2],
                        index_buffer: self.index_buffer,
                        input_layout_hash: self.input_layout,
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
