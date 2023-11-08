use crate::entity::EPrimitiveType;
use crate::render::vertex_buffers::load_vertex_buffers;
use crate::statics::{Unk80807193, Unk80807194, Unk808071a7};

use anyhow::ensure;
use destiny_pkg::TagHash;

use itertools::Itertools;

use crate::packages::package_manager;

use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;

use super::drawcall::{
    ConstantBufferBinding, DrawCall, GeometryType, ShadingTechnique, SortValue3d, Transparency,
};
use super::renderer::Renderer;

pub struct StaticModelBuffer {
    vertex_buffer1: TagHash,
    vertex_buffer2: TagHash,
    color_buffer: TagHash,

    index_buffer: TagHash,
    input_layout: u64,
}

pub struct StaticModel {
    pub buffers: Vec<StaticModelBuffer>,

    pub overlay_models: Vec<StaticOverlayModel>,

    pub subheader: Unk80807194,

    model: Unk808071a7,
}

impl StaticModel {
    pub fn load(
        model: Unk808071a7,
        renderer: &Renderer,
        _model_hash: TagHash,
    ) -> anyhow::Result<StaticModel> {
        let pm = package_manager();
        let header: Unk80807194 = pm.read_tag_struct(model.unk8).unwrap();

        ensure!(header.mesh_groups.len() == model.materials.len());

        let mut buffers = vec![];
        for (
            buffer_index,
            (index_buffer_hash, vertex_buffer_hash, vertex2_buffer_hash, color_buffer_hash),
        ) in header.buffers.iter().enumerate()
        {
            // let vertex_header: VertexBufferHeader =
            //     pm.read_tag_struct(*vertex_buffer_hash).unwrap();

            // if vertex_header.stride == 24 || vertex_header.stride == 48 {
            //     warn!("Support for 32-bit floats in vertex buffers are disabled");
            //     continue;
            // }

            // let t = pm.get_entry(*vertex_buffer_hash).unwrap().reference;

            // let vertex_data = pm.read_tag(t).unwrap();

            // let mut vertex2_stride = None;
            // let mut vertex2_data = None;
            // if vertex2_buffer_hash.is_some() {
            //     let vertex2_header: VertexBufferHeader =
            //         pm.read_tag_struct(*vertex2_buffer_hash).unwrap();
            //     let t = pm.get_entry(*vertex2_buffer_hash).unwrap().reference;

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
            //     device
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
            // unsafe {
            //     let name = format!("VB {} (model {})\0", vertex_buffer_hash, model_hash);
            //     combined_vertex_buffer
            //         .SetPrivateData(
            //             &WKPDID_D3DDebugObjectName,
            //             name.len() as u32 - 1,
            //             Some(name.as_ptr() as _),
            //         )
            //         .expect("Failed to set VS name")
            // };

            renderer.render_data.load_buffer(*index_buffer_hash, false);
            renderer.render_data.load_buffer(*vertex_buffer_hash, false);
            renderer
                .render_data
                .load_buffer(*vertex2_buffer_hash, false);
            renderer.render_data.load_buffer(*color_buffer_hash, true);

            for m in &model.materials {
                renderer.render_data.load_material(renderer, *m);
            }

            // Find the first normal material to use for the input layout
            let mut buffer_layout_material = TagHash(u32::MAX);
            for (iu, u) in header.mesh_groups.iter().enumerate() {
                let p = &header.parts[u.part_index as usize];

                if p.buffer_index == buffer_index as u8 {
                    let material = model.materials[iu];
                    if let Some(mat) = renderer.render_data.data().materials.get(&material) {
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
                    if let Some(mat) = renderer.render_data.data().materials.get(material) {
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

    pub fn draw(
        &self,
        renderer: &Renderer,
        instance_buffer: ID3D11Buffer,
        instance_count: usize,
        draw_transparent: bool,
    ) -> anyhow::Result<()> {
        if draw_transparent {
            for u in &self.overlay_models {
                u.draw(renderer, instance_buffer.clone(), instance_count);
            }
        } else {
            for (iu, u) in self
                .subheader
                .mesh_groups
                .iter()
                .enumerate()
                .filter(|(_, u)| u.unk2 == 0)
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
                            .with_technique(ShadingTechnique::Deferred)
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
                            index_start: p.index_start,
                            index_count: p.index_count,
                            instance_start: None,
                            instance_count: Some(instance_count as _),
                            primitive_type: p.primitive_type.to_dx(),
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
    model: Unk80807193,
}

impl StaticOverlayModel {
    pub fn load(model: Unk80807193, renderer: &Renderer) -> anyhow::Result<StaticOverlayModel> {
        let _pm = package_manager();

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

    pub fn draw(&self, renderer: &Renderer, instance_buffer: ID3D11Buffer, instance_count: usize) {
        if !self.model.lod.is_highest_detail() {
            return;
        }
        let technique = renderer
            .render_data
            .data()
            .material_shading_technique(self.model.material)
            .unwrap_or(ShadingTechnique::Forward);

        renderer.push_drawcall(
            SortValue3d::empty()
                // TODO(cohae): calculate depth (need to draw instances separately)
                .with_depth(u32::MAX)
                .with_material(self.model.material.0)
                .with_transparency(if technique == ShadingTechnique::Deferred {
                    Transparency::None
                } else {
                    Transparency::Additive
                })
                .with_technique(technique)
                .with_geometry_type(GeometryType::StaticDecal),
            DrawCall {
                vertex_buffers: vec![self.buffers.vertex_buffer1, self.buffers.vertex_buffer2],
                index_buffer: self.buffers.index_buffer,
                color_buffer: Some(self.buffers.color_buffer),
                input_layout_hash: self.buffers.input_layout,
                buffer_bindings: vec![ConstantBufferBinding::new(1, instance_buffer)],
                variant_material: None,
                index_start: self.model.index_start,
                index_count: self.model.index_count,
                instance_start: None,
                instance_count: Some(instance_count as _),
                primitive_type: self.model.primitive_type.to_dx(),
            },
        );
    }
}
