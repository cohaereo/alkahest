use alkahest_data::{
    statics::{SStaticMesh, SStaticMeshData, SStaticMeshOverlay},
    tfx::TfxRenderStage,
};
use anyhow::ensure;
use destiny_pkg::TagHash;
use hecs::Entity;
use itertools::Itertools;
use tiger_parse::PackageManagerExt;
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
use crate::{packages::package_manager, render::vertex_buffers::load_vertex_buffers};

pub struct StaticModelBuffer {
    pub vertex_buffer1: TagHash,
    pub vertex_buffer2: TagHash,
    pub color_buffer: TagHash,

    index_buffer: TagHash,
    input_layout: u64,
}

pub struct StaticModel {
    pub buffers: Vec<StaticModelBuffer>,

    pub overlay_models: Vec<StaticOverlayModel>,

    pub subheader: SStaticMeshData,

    model: SStaticMesh,
}

impl StaticModel {
    pub fn load(model: SStaticMesh, renderer: &Renderer) -> anyhow::Result<StaticModel> {
        let pm = package_manager();
        let header: SStaticMeshData = pm.read_tag_struct(model.unk8).unwrap();

        ensure!(header.mesh_groups.len() == model.materials.len());
        let mut buffers = vec![];
        for (
            buffer_index,
            (index_buffer_hash, vertex_buffer_hash, vertex2_buffer_hash, color_buffer_hash),
        ) in header.buffers.iter().enumerate()
        {
            renderer.render_data.load_buffer(*index_buffer_hash, false);
            renderer.render_data.load_buffer(*vertex_buffer_hash, false);
            renderer
                .render_data
                .load_buffer(*vertex2_buffer_hash, false);
            renderer.render_data.load_buffer(*color_buffer_hash, true);

            for m in &model.materials {
                renderer.render_data.load_technique(renderer, *m);
            }

            // Find the first normal material to use for the input layout
            let mut buffer_layout_material = TagHash(u32::MAX);
            for (iu, u) in header.mesh_groups.iter().enumerate() {
                let p = &header.parts[u.part_index as usize];

                if p.buffer_index == buffer_index as u8 {
                    let material = model.materials[iu];
                    if let Some(mat) = renderer.render_data.data().techniques.get(&material) {
                        if mat.unk8 == 1 {
                            buffer_layout_material = material;
                            break;
                        }
                    }
                }
            }

            // Fall back to any working material in the material array
            if !buffer_layout_material.is_some() {
                for material in &model.materials {
                    if let Some(mat) = renderer.render_data.data().techniques.get(material) {
                        if mat.unk8 == 1 {
                            buffer_layout_material = *material;
                            break;
                        }
                    }
                }
            }

            let input_layout = load_vertex_buffers(
                renderer,
                buffer_layout_material,
                &[*vertex_buffer_hash, *vertex2_buffer_hash],
            )?;

            buffers.push(StaticModelBuffer {
                vertex_buffer1: *vertex_buffer_hash,
                vertex_buffer2: *vertex2_buffer_hash,
                index_buffer: *index_buffer_hash,
                color_buffer: *color_buffer_hash,
                input_layout,
            })
        }

        Ok(StaticModel {
            overlay_models: model
                .unk20
                .iter()
                .map(|m| {
                    let r = StaticOverlayModel::load(m.clone(), renderer);
                    if let Err(e) = &r {
                        error!("Failed to load static overlay mesh: {e}");
                    }

                    r
                })
                .filter_map(Result::ok)
                .collect_vec(),
            buffers,
            model,
            subheader: header,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        renderer: &Renderer,
        instance_buffer: ID3D11Buffer,
        instance_count: usize,
        draw_opaque: bool,
        draw_transparent: bool,
        draw_decals: bool,
        entity: Entity,
    ) -> anyhow::Result<()> {
        for u in &self.overlay_models {
            u.draw(
                renderer,
                instance_buffer.clone(),
                instance_count,
                draw_transparent,
                draw_decals,
                entity,
            );
        }

        if draw_opaque {
            for (iu, u) in self
                .subheader
                .mesh_groups
                .iter()
                .enumerate()
                .filter(|(_, u)| u.render_stage == TfxRenderStage::GenerateGbuffer)
            {
                let p = &self.subheader.parts[u.part_index as usize];
                if !p.lod_category.is_highest_detail() {
                    continue;
                }

                if let Some(buffers) = self.buffers.get(p.buffer_index as usize) {
                    let material = self.model.materials[iu];

                    renderer.push_drawcall(
                        SortValue3d::empty()
                            // TODO(cohae): calculate depth (need to draw instances separately)
                            .with_depth(u32::MIN)
                            .with_material(material.0)
                            .with_shading_mode(ShadingMode::Deferred)
                            .with_transparency(Transparency::None)
                            .with_geometry_type(GeometryType::Static),
                        DrawCall {
                            vertex_buffers: vec![buffers.vertex_buffer1, buffers.vertex_buffer2],
                            index_buffer: buffers.index_buffer,
                            color_buffer: Some(buffers.color_buffer),
                            input_layout_hash: buffers.input_layout,
                            buffer_bindings: vec![ConstantBufferBinding::new(
                                1,
                                instance_buffer.clone(),
                            )],
                            variant_material: None,
                            dyemap: None,
                            index_start: p.index_start,
                            index_count: p.index_count,
                            instance_start: None,
                            instance_count: Some(instance_count as _),
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
        }

        Ok(())
    }
}

pub struct StaticOverlayModel {
    buffers: StaticModelBuffer,
    model: SStaticMeshOverlay,
}

impl StaticOverlayModel {
    pub fn load(
        model: SStaticMeshOverlay,
        renderer: &Renderer,
    ) -> anyhow::Result<StaticOverlayModel> {
        renderer.render_data.load_buffer(model.index_buffer, false);
        renderer.render_data.load_buffer(model.vertex_buffer, false);
        renderer
            .render_data
            .load_buffer(model.vertex_buffer2, false);
        renderer.render_data.load_buffer(model.color_buffer, true);

        let input_layout = load_vertex_buffers(
            renderer,
            model.material,
            &[model.vertex_buffer, model.vertex_buffer2],
        )?;

        Ok(Self {
            buffers: StaticModelBuffer {
                vertex_buffer1: model.vertex_buffer,
                vertex_buffer2: model.vertex_buffer2,
                index_buffer: model.index_buffer,
                color_buffer: model.color_buffer,
                input_layout,
            },
            model,
        })
    }

    pub fn draw(
        &self,
        renderer: &Renderer,
        instance_buffer: ID3D11Buffer,
        instance_count: usize,
        draw_transparent: bool,
        draw_decals: bool,
        entity: Entity,
    ) {
        if !self.model.lod.is_highest_detail() {
            return;
        }

        let shading_mode = ShadingMode::from_tfx_render_stage(self.model.render_stage);

        if !draw_decals && self.model.render_stage == TfxRenderStage::Decals {
            return;
        }

        if self.model.render_stage == TfxRenderStage::LightShaftOcclusion {
            return;
        }

        if !draw_transparent && shading_mode == ShadingMode::Forward {
            return;
        }

        renderer.push_drawcall(
            SortValue3d::empty()
                // TODO(cohae): calculate depth (need to draw instances separately)
                .with_depth(u32::MAX)
                .with_material(self.model.material.0)
                .with_transparency(if shading_mode == ShadingMode::Deferred {
                    Transparency::None
                } else {
                    Transparency::Additive
                })
                .with_shading_mode(shading_mode)
                .with_geometry_type(GeometryType::StaticDecal),
            DrawCall {
                vertex_buffers: vec![self.buffers.vertex_buffer1, self.buffers.vertex_buffer2],
                index_buffer: self.buffers.index_buffer,
                color_buffer: Some(self.buffers.color_buffer),
                input_layout_hash: self.buffers.input_layout,
                buffer_bindings: vec![ConstantBufferBinding::new(1, instance_buffer)],
                variant_material: None,
                dyemap: None,
                index_start: self.model.index_start,
                index_count: self.model.index_count,
                instance_start: None,
                instance_count: Some(instance_count as _),
                primitive_type: match self.model.primitive_type {
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
