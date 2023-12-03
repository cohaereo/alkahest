use crate::entity::EPrimitiveType;
use crate::render::vertex_buffers::load_vertex_buffers;
use crate::statics::{Unk80807193, Unk80807194, Unk8080719a, Unk8080719b, Unk808071a7};

use anyhow::ensure;
use destiny_pkg::TagHash;
use glam::{Mat4, Vec3};
use itertools::Itertools;

use crate::packages::package_manager;

use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;

use super::drawcall::{DrawCall, ShadingTechnique, SortValue3d, Transparency};
use super::renderer::Renderer;

pub struct StaticModelBuffer {
    vertex_buffer1: TagHash,
    vertex_buffer2: TagHash,

    index_buffer: TagHash,
    input_layout: u64,
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
        renderer: &Renderer,
        _model_hash: TagHash,
    ) -> anyhow::Result<StaticModel> {
        let pm = package_manager();
        let header: Unk80807194 = pm.read_tag_struct(model.unk8).unwrap();

        ensure!(header.mesh_groups.len() == model.materials.len());

        let mut buffers = vec![];
        for (buffer_index, (index_buffer_hash, vertex_buffer_hash, vertex2_buffer_hash, _u3)) in
            header.buffers.iter().enumerate()
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
            // if vertex2_buffer_hash.is_valid() {
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

            renderer.render_data.load_buffer(*index_buffer_hash);
            renderer.render_data.load_buffer(*vertex_buffer_hash);
            renderer.render_data.load_buffer(*vertex2_buffer_hash);

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
            if !buffer_layout_material.is_valid() {
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
                input_layout,
            })
        }

        Ok(StaticModel {
            decal_parts: model
                .unk20
                .iter()
                .map(|m| StaticOverlayModel::load(m.clone(), device, renderer).unwrap())
                .collect_vec(),
            buffers,
            model,
            parts: header.parts.to_vec(),
            mesh_groups: header.mesh_groups.to_vec(),
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

                    let material = self.model.materials[iu];

                    renderer.push_drawcall(
                        SortValue3d::empty()
                            // TODO(cohae): calculate depth (need to draw instances separately)
                            .with_depth(u32::MIN)
                            .with_material(material.0)
                            .with_technique(ShadingTechnique::Deferred)
                            .with_transparency(Transparency::Blend),
                        DrawCall {
                            vertex_buffers: vec![buffers.vertex_buffer1, buffers.vertex_buffer2],
                            index_buffer: buffers.index_buffer,
                            input_layout_hash: buffers.input_layout,
                            cb11: Some(instance_buffer.clone()),
                            dyemap: None,
                            variant_material: None,
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
    model: Unk80807193,
}

impl StaticOverlayModel {
    pub fn load(
        model: Unk80807193,
        _device: &ID3D11Device,
        renderer: &Renderer,
    ) -> anyhow::Result<StaticOverlayModel> {
        let _pm = package_manager();
        // let vertex_header: VertexBufferHeader = pm.read_tag_struct(model.vertex_buffer).unwrap();

        // if vertex_header.stride == 24 || vertex_header.stride == 48 {
        //     anyhow::bail!("Support for 32-bit floats in vertex buffers are disabled");
        // }

        // let t = pm.get_entry(model.vertex_buffer).unwrap().reference;

        // let vertex_data = pm.read_tag(t).unwrap();

        // let mut vertex2_stride = None;
        // let mut vertex2_data = None;
        // if model.vertex_buffer2.is_valid() {
        //     let vertex2_header: VertexBufferHeader =
        //         pm.read_tag_struct(model.vertex_buffer2).unwrap();
        //     let t = pm.get_entry(model.vertex_buffer2).unwrap().reference;

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

        renderer.render_data.load_buffer(model.index_buffer);
        renderer.render_data.load_buffer(model.vertex_buffer);
        renderer.render_data.load_buffer(model.vertex_buffer2);

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
                input_layout,
            },
            model,
        })
    }

    pub fn draw(
        &self,
        renderer: &mut Renderer,
        instance_buffer: ID3D11Buffer,
        instance_count: usize,
    ) {
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
                .with_technique(technique)
                .with_transparency(Transparency::Additive),
            DrawCall {
                vertex_buffers: vec![self.buffers.vertex_buffer1, self.buffers.vertex_buffer2],
                index_buffer: self.buffers.index_buffer,
                input_layout_hash: self.buffers.input_layout,
                cb11: Some(instance_buffer),
                dyemap: None,
                variant_material: None,
                index_start: self.model.index_start,
                index_count: self.model.index_count,
                instance_start: None,
                instance_count: Some(instance_count as _),
                primitive_type: D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
            },
        );
    }
}
