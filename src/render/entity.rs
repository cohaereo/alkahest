use destiny_pkg::TagHash;

use glam::Vec4;

use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;

use crate::entity::EPrimitiveType;
use crate::entity::Unk808072c5;
use crate::entity::Unk8080737e;
use crate::entity::Unk808073a5;

use crate::packages::package_manager;
use crate::render::vertex_buffers::load_vertex_buffers;

use super::drawcall::ConstantBufferBinding;
use super::drawcall::DrawCall;
use super::drawcall::ShadingTechnique;
use super::drawcall::SortValue3d;
use super::drawcall::Transparency;
use super::renderer::Renderer;
use super::DeviceContextSwapchain;

pub struct EntityModelBuffer {
    vertex_buffer1: TagHash,
    vertex_buffer2: TagHash,
    color_buffer: TagHash,

    index_buffer: TagHash,
    input_layout: u64,
}

pub struct EntityRenderer {
    meshes: Vec<(EntityModelBuffer, Vec<Unk8080737e>)>,

    _material_map: Vec<Unk808072c5>,
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
            let _pm = package_manager();
            // let vertex_header: VertexBufferHeader =
            //     pm.read_tag_struct(mesh.vertex_buffer1).unwrap();

            // if vertex_header.stride == 24 || vertex_header.stride == 48 {
            //     panic!("Support for 32-bit floats in vertex buffers are disabled");
            // }

            // let t = pm.get_entry(mesh.vertex_buffer1).unwrap().reference;

            // let vertex_data = pm.read_tag(t).unwrap();

            // let mut vertex2_stride = None;
            // let mut vertex2_data = None;
            // if mesh.vertex_buffer2.is_valid() {
            //     let vertex2_header: VertexBufferHeader =
            //         pm.read_tag_struct(mesh.vertex_buffer2).unwrap();
            //     let t = pm.get_entry(mesh.vertex_buffer2).unwrap().reference;

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

            renderer.render_data.load_buffer(mesh.index_buffer, false);
            renderer.render_data.load_buffer(mesh.vertex_buffer1, false);
            renderer.render_data.load_buffer(mesh.vertex_buffer2, false);
            renderer.render_data.load_buffer(mesh.color_buffer, true);

            let input_layout = load_vertex_buffers(
                renderer,
                mesh.parts
                    .iter()
                    .find(|v| v.material.is_valid())
                    .map(|v| v.material)
                    .or_else(|| materials.first().cloned())
                    .unwrap(),
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
            _material_map: material_map,
            materials,
            model,
        })
    }

    fn get_variant_material(&self, index: u16) -> Option<TagHash> {
        // let map = self.material_map.iter().find(|&v| {
        //     (v.material_start as isize..v.material_start as isize + v.material_count as isize)
        //         .contains(&(index as isize))
        // })?;

        // None
        // self.materials.first().cloned()
        // self.material_map
        //     .get(index as usize)
        //     .map(|m| self.materials.get((m.material_start) as usize))
        //     .flatten()
        //     .cloned()

        if index == u16::MAX {
            None
        } else {
            // self.materials
            //     .get(fastrand::usize(0..self.materials.len()))
            //     .cloned()
            self.materials.first().cloned()
        }
    }

    pub fn draw(&self, renderer: &mut Renderer, cb11: ID3D11Buffer) -> anyhow::Result<()> {
        for (buffers, parts) in self.meshes.iter() {
            for p in parts {
                if !p.lod_category.is_highest_detail() {
                    continue;
                }

                let variant_material = self.get_variant_material(p.variant_shader_index);

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
                        .with_technique(shading_technique),
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
