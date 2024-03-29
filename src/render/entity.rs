use alkahest_data::entity::{SEntityModel, Unk808072c5, Unk8080737e};
use anyhow::Context;
use destiny_pkg::TagHash;
use glam::Vec4;
use hecs::Entity;
use windows::Win32::Graphics::{
    Direct3D::{D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP},
    Direct3D11::*,
};

use super::{
    drawcall::{
        ConstantBufferBinding, DrawCall, GeometryType, ShadingMode, SortValue3d, Transparency,
    },
    renderer::Renderer,
};
use crate::render::vertex_buffers::load_vertex_buffers;

#[derive(Clone)]
pub struct EntityModelBuffer {
    vertex_buffer1: TagHash,
    vertex_buffer2: TagHash,
    color_buffer: TagHash,

    index_buffer: TagHash,
    input_layout: u64,
}

#[derive(Clone)]
pub struct EntityRenderer {
    meshes: Vec<(EntityModelBuffer, Vec<Unk8080737e>)>,

    material_map: Vec<Unk808072c5>,
    materials: Vec<TagHash>,

    model: SEntityModel,
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
        model: SEntityModel,
        material_map: Vec<Unk808072c5>,
        materials: Vec<TagHash>,
        renderer: &Renderer,
    ) -> anyhow::Result<Self> {
        let mut meshes = vec![];

        for mesh in &model.meshes {
            renderer.render_data.load_buffer(mesh.index_buffer, false);
            renderer.render_data.load_buffer(mesh.vertex_buffer1, false);
            renderer.render_data.load_buffer(mesh.vertex_buffer2, false);
            renderer.render_data.load_buffer(mesh.color_buffer, true);

            let input_layout = load_vertex_buffers(
                renderer,
                mesh.parts
                    .iter()
                    .find(|v| v.material.is_some())
                    .map(|v| v.material)
                    .or_else(|| materials.first().cloned())
                    .context("Can't find a material to create an input layout!")?,
                &[mesh.vertex_buffer1, mesh.vertex_buffer2],
            )?;

            meshes.push((
                EntityModelBuffer {
                    vertex_buffer1: mesh.vertex_buffer1,
                    vertex_buffer2: mesh.vertex_buffer2,
                    index_buffer: mesh.index_buffer,
                    color_buffer: mesh.color_buffer,
                    input_layout,
                },
                mesh.parts.to_vec(),
            ))
        }

        Ok(Self {
            meshes,
            material_map,
            materials,
            model,
        })
    }

    fn get_variant_material(&self, index: u16, variant: usize) -> Option<TagHash> {
        if index == u16::MAX {
            None
        } else {
            let variant_range = &self.material_map[index as usize];
            Some(
                self.materials[variant_range.material_start as usize
                    + (variant % variant_range.material_count as usize)],
            )
        }
    }

    pub fn draw(
        &self,
        renderer: &Renderer,
        cb11: ID3D11Buffer,
        entity: Entity,
    ) -> anyhow::Result<()> {
        for (buffers, parts) in self.meshes.iter() {
            for p in parts {
                if !p.lod_category.is_highest_detail() {
                    continue;
                }

                let variant_material = self.get_variant_material(p.variant_shader_index, 0);

                let shading_technique = renderer
                    .render_data
                    .data()
                    .material_shading_technique(variant_material.unwrap_or(p.material))
                    .unwrap_or(ShadingMode::Forward);

                renderer.push_drawcall(
                    SortValue3d::empty()
                        // TODO(cohae): calculate depth (need to draw instances separately)
                        .with_depth(u32::MAX)
                        .with_material(p.material.0)
                        .with_transparency(if shading_technique == ShadingMode::Deferred {
                            Transparency::None
                        } else {
                            Transparency::Additive
                        })
                        .with_shading_mode(shading_technique)
                        .with_geometry_type(GeometryType::Entity),
                    DrawCall {
                        vertex_buffers: vec![buffers.vertex_buffer1, buffers.vertex_buffer2],
                        index_buffer: buffers.index_buffer,
                        color_buffer: Some(buffers.color_buffer),
                        input_layout_hash: buffers.input_layout,
                        buffer_bindings: vec![ConstantBufferBinding::new(1, cb11.clone())],
                        variant_material,
                        dyemap: None,
                        index_start: p.index_start,
                        index_count: p.index_count,
                        instance_start: None,
                        instance_count: None,
                        primitive_type: match p.primitive_type {
                            alkahest_data::geometry::EPrimitiveType::Triangles => {
                                D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST
                            }
                            alkahest_data::geometry::EPrimitiveType::TriangleStrip => {
                                D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP
                            }
                        },
                        entity,
                    },
                );
            }
        }

        Ok(())
    }
}
