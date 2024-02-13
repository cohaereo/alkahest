use std::sync::Arc;

use destiny_pkg::TagHash;
use glam::{Mat4, Vec4};
use hecs::Entity;
use windows::Win32::Graphics::Direct3D::D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP;

use super::{
    drawcall::{
        ConstantBufferBinding, DrawCall, GeometryType, ShadingMode, SortValue3d, Transparency,
    },
    renderer::Renderer,
    vertex_buffers::load_vertex_buffers,
    ConstantBuffer, DeviceContextSwapchain,
};
use crate::map::STerrain;

pub struct TerrainRenderer {
    terrain: STerrain,
    group_cbuffers: Vec<ConstantBuffer<Mat4>>,

    vertex_buffer1: TagHash,
    vertex_buffer2: TagHash,
    input_layout: u64,

    index_buffer: TagHash,
}

impl TerrainRenderer {
    pub fn load(
        terrain: STerrain,
        dcs: Arc<DeviceContextSwapchain>,
        renderer: &Renderer,
    ) -> anyhow::Result<TerrainRenderer> {
        renderer.render_data.load_buffer(terrain.indices, false);
        renderer
            .render_data
            .load_buffer(terrain.vertex_buffer, false);
        renderer
            .render_data
            .load_buffer(terrain.vertex_buffer2, false);

        let mut group_cbuffers = vec![];
        for group in &terrain.mesh_groups {
            let offset = Vec4::new(
                terrain.unk30.x,
                terrain.unk30.y,
                terrain.unk30.z,
                terrain.unk30.w,
            );

            let texcoord_transform =
                Vec4::new(group.unk20.x, group.unk20.y, group.unk20.z, group.unk20.w);

            let scope_terrain = Mat4::from_cols(offset, texcoord_transform, Vec4::ZERO, Vec4::ZERO);

            let cb = ConstantBuffer::create(dcs.clone(), Some(&scope_terrain))?;
            group_cbuffers.push(cb);
        }

        for p in &terrain.mesh_parts {
            renderer.render_data.load_technique(renderer, p.material);
        }

        // Find the first normal material to use for the input layout
        let mut buffer_layout_material = TagHash(u32::MAX);
        for m in terrain.mesh_parts.iter() {
            if let Some(mat) = renderer.render_data.data().techniques.get(&m.material) {
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

    pub fn draw(&self, renderer: &Renderer, entity: Entity) -> anyhow::Result<()> {
        for part in self.terrain.mesh_parts.iter()
        // .filter(|u| u.detail_level == 0)
        {
            if let Some(group) = self.terrain.mesh_groups.get(part.group_index as usize) {
                let cb11 = &self.group_cbuffers[part.group_index as usize];

                renderer.push_drawcall(
                    SortValue3d::empty()
                        // TODO(cohae): calculate depth
                        .with_depth(u32::MIN)
                        .with_material(part.material.0)
                        .with_shading_mode(ShadingMode::Deferred)
                        .with_transparency(Transparency::None)
                        .with_geometry_type(GeometryType::Terrain),
                    DrawCall {
                        vertex_buffers: vec![self.vertex_buffer1, self.vertex_buffer2],
                        index_buffer: self.index_buffer,
                        color_buffer: None,
                        input_layout_hash: self.input_layout,
                        buffer_bindings: vec![ConstantBufferBinding::new(
                            11,
                            cb11.buffer().clone(),
                        )],
                        dyemap: Some(group.dyemap),
                        variant_material: None,
                        index_start: part.index_start,
                        index_count: part.index_count as _,
                        instance_start: None,
                        instance_count: None,
                        primitive_type: D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP,
                        entity,
                    },
                );
            } else {
                panic!("Could not get terrain mesh group {}", part.group_index)
            }
        }

        Ok(())
    }
}
