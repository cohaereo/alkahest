use anyhow::Context;
use destiny_pkg::TagHash;

use glam::Vec4;

use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;

use crate::entity::EPrimitiveType;
use crate::entity::Unk808072c5;
use crate::entity::Unk8080737e;

use crate::entity::Unk808073a5;
use crate::render::vertex_buffers::load_vertex_buffers;

use super::drawcall::ConstantBufferBinding;
use super::drawcall::DrawCall;
use super::drawcall::GeometryType;
use super::drawcall::ShadingTechnique;
use super::drawcall::SortValue3d;
use super::drawcall::Transparency;
use super::renderer::Renderer;
use super::DeviceContextSwapchain;

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
        renderer: &Renderer,
        _dcs: &DeviceContextSwapchain,
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
            Some(self.materials[variant_range.material_start as usize + variant])
        }
    }

    pub fn draw(&self, renderer: &Renderer, cb11: ID3D11Buffer) -> anyhow::Result<()> {
        for (buffers, parts) in self.meshes.iter() {
            for p in parts {
                if !p.lod_category.is_highest_detail() {
                    continue;
                }

                let variant_material = self.get_variant_material(p.variant_shader_index, 0);

                let primitive_type = match p.primitive_type {
                    EPrimitiveType::Triangles => D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
                    EPrimitiveType::TriangleStrip => D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP,
                };

                let shading_technique = renderer
                    .render_data
                    .data()
                    .material_shading_technique(variant_material.unwrap_or(p.material))
                    .unwrap_or(ShadingTechnique::Forward);

                renderer.push_drawcall(
                    SortValue3d::empty()
                        // TODO(cohae): calculate depth (need to draw instances separately)
                        .with_depth(u32::MAX)
                        .with_material(p.material.0)
                        .with_transparency(if shading_technique == ShadingTechnique::Deferred {
                            Transparency::None
                        } else {
                            Transparency::Additive
                        })
                        .with_technique(shading_technique)
                        .with_geometry_type(GeometryType::Entity),
                    DrawCall {
                        vertex_buffers: vec![buffers.vertex_buffer1, buffers.vertex_buffer2],
                        index_buffer: buffers.index_buffer,
                        color_buffer: Some(buffers.color_buffer),
                        input_layout_hash: buffers.input_layout,
                        buffer_bindings: vec![ConstantBufferBinding::new(1, cb11.clone())],
                        variant_material,
                        index_start: p.index_start,
                        index_count: p.index_count,
                        instance_start: None,
                        instance_count: None,
                        primitive_type,
                    },
                );
            }
        }

        Ok(())
    }
}
