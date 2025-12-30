use std::sync::Arc;

use alkahest_core::job::{potassium::JobHandle, SCHEDULER};
use alkahest_data::tfx::{
    features::{
        ao::SStaticAmbientOcclusion,
        dynamic::RenderStageSubscription,
        terrain::{STerrain, TerrainDetailLevel},
    },
    RenderStage, ShaderStage,
};
use anyhow::Context;
use glam::Vec4;
use tiger_parse::PackageManagerExt;
use tiger_pkg::{package_manager, TagHash};

use super::FeatureRenderer;
use crate::{
    asset::{index_buffer::IndexBuffer, texture::Texture, vertex_buffer::VertexBuffer, Handle},
    camera::Camera,
    gpu::{cbuffer::ConstantBuffer, command_list::CommandList},
    gpu_span,
    tfx::technique::Technique,
    Gpu, Renderer,
};

#[repr(C)]
#[derive(Default)]
pub struct TerrainPatchGroupConstants {
    offset: Vec4,
    texcoord_transform: Vec4,
    unk20: f32,
    unk24: f32,
    unk28: f32,
    ao_offset: u32,
    unk30: Vec4,
}

pub struct TerrainPatchesRenderer {
    terrain: STerrain,
    techniques: Vec<Handle<Technique>>,
    dyemaps: Vec<Handle<Texture>>,
    group_cbuffers: Vec<ConstantBuffer<TerrainPatchGroupConstants>>,
    constants_dirty: bool,
    detail_level: TerrainDetailLevel,

    pub vertex0_buffer: Handle<VertexBuffer>,
    pub vertex1_buffer: Handle<VertexBuffer>,
    pub index_buffer: Handle<IndexBuffer>,

    pub hash: TagHash,
    pub identifier: u64,
}

impl TerrainPatchesRenderer {
    pub fn load(gpu: &Arc<Gpu>, hash: TagHash, identifier: u64) -> anyhow::Result<Box<Self>> {
        let terrain: STerrain = package_manager().read_tag_struct(hash)?;

        let assets = &Renderer::instance().asset_manager;
        let dyemaps = terrain
            .mesh_groups
            .iter()
            .map(|group| assets.load(group.dyemap))
            .collect();

        let techniques = terrain
            .mesh_parts
            .iter()
            .map(|part| assets.load(part.technique))
            .collect();

        let group_cbuffers = (0..terrain.mesh_groups.len())
            .map(|_| ConstantBuffer::create(gpu, None))
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to create group constant buffers")?;

        Ok(Box::new(Self {
            vertex0_buffer: assets.load(terrain.vertex0_buffer),
            vertex1_buffer: assets.load(terrain.vertex1_buffer),
            index_buffer: assets.load(terrain.index_buffer),
            constants_dirty: true,
            detail_level: TerrainDetailLevel::Medium,
            terrain,
            techniques,
            dyemaps,
            group_cbuffers,
            hash,
            identifier,
        }))
    }

    #[profiling::function]
    pub fn render(&self, cmd: &mut CommandList, _render_stage: RenderStage) {
        // gpu_event!(renderer.gpu, format!("terrain_patch {}", self.hash));
        gpu_span!();

        // Layout 22(tfs/mara)/60(sk)
        //  - int4 v0 : POSITION0, // Format DXGI_FORMAT_R16G16B16A16_SINT size 8
        //  - float4 v1 : NORMAL0, // Format DXGI_FORMAT_R16G16B16A16_SNORM size 8
        //  - float2 v2 : TEXCOORD1, // Format DXGI_FORMAT_R16G16_FLOAT size 4
        cmd.set_input_layout(22);
        cmd.set_input_topology(alkahest_data::tfx::PrimitiveType::TriangleStrip);

        if let (Some(vertex0), Some(vertex1), Some(index)) = (
            self.vertex0_buffer.get(),
            self.vertex1_buffer.get(),
            self.index_buffer.get(),
        ) {
            index.bind(cmd);
            cmd.input_assembler_set_vertex_buffers(
                0,
                &[Some(&vertex0.buffer), Some(&vertex1.buffer)],
                Some(&[vertex0.stride as _, vertex1.stride as _]),
                Some(&[0, 0]),
            )
            .unwrap();
        } else {
            return;
        }

        for (i, part) in self
            .terrain
            .mesh_parts
            .iter()
            .enumerate()
            .filter(|(_, u)| u.detail_level == self.detail_level)
        {
            let cb11 = &self.group_cbuffers[part.group_index as usize];

            cb11.bind(cmd, ShaderStage::Vertex, 11);
            if let Some(dyemap) = self.dyemaps[part.group_index as usize].get() {
                dyemap.bind(cmd, 14, alkahest_data::tfx::ShaderStage::Pixel);
            }

            if let Some(technique) = self.techniques[i].get() {
                technique.bind(cmd).expect("Failed to bind technique");
            } else {
                continue;
            }

            cmd.draw_indexed(part.index_count as _, part.index_start as _, 0);
        }
    }

    #[profiling::function]
    pub fn update_constants(
        &self,
        ctx: &d3d11::DeviceContext,
        ao: Option<&SStaticAmbientOcclusion>,
    ) {
        if ao
            .and_then(|ao| ao.get_offset_by_identifier(self.identifier))
            .is_none()
        {
            warn!("No AO for terrain 0x{:016X}", self.identifier);
        }

        for (i, group) in self.terrain.mesh_groups.iter().enumerate() {
            let offset = Vec4::new(
                self.terrain.unk30.x,
                self.terrain.unk30.y,
                self.terrain.unk30.z,
                self.terrain.unk30.w,
            );

            let texcoord_transform =
                Vec4::new(group.unk20.x, group.unk20.y, group.unk20.z, group.unk20.w);

            // let scope_terrain = Mat4::from_cols(offset, texcoord_transform, Vec4::ZERO, Vec4::ZERO);
            let scope_terrain = TerrainPatchGroupConstants {
                offset,
                texcoord_transform,
                ao_offset: ao
                    .and_then(|ao| ao.get_offset_by_identifier(self.identifier))
                    .unwrap_or_default(),
                ..Default::default()
            };

            self.group_cbuffers[i].write(ctx, &scope_terrain).ok();
        }
    }
}

impl FeatureRenderer for TerrainPatchesRenderer {
    fn visibility_test(&mut self, camera: &Camera) -> bool {
        let center = self.terrain.bounds.center();
        let _radius = self.terrain.bounds.radius();
        let _distance = camera.position.distance(center);
        // self.detail_level = match distance {
        //     d if d > radius * 4.0 => TerrainDetailLevel::Low,
        //     d if d > radius * 2.0 => TerrainDetailLevel::Medium,
        //     _ => TerrainDetailLevel::High,
        // };

        camera
            .culling_frustum
            .aabb_intersecting(&self.terrain.bounds)
    }

    fn extract_and_prepare(&mut self, renderer: &Renderer, _extracted_data: &dyn std::any::Any) {
        if self.constants_dirty {
            self.update_constants(&renderer.gpu.context(), renderer.ao.read().as_ref());
            self.constants_dirty = false;
        }
    }

    fn submit(&self, cmd: &mut CommandList, stage: RenderStage) {
        let renderer = Renderer::instance();
        if let Some(ao_vb) = renderer.ao_buffer.lock().as_ref().and_then(|h| h.get()) {
            cmd.vertex_set_shader_resources(1, std::slice::from_ref(&ao_vb.srv.as_ref()));
        }

        self.render(cmd, stage);
    }

    fn submit_parallel(
        &self,
        _renderer: &Arc<Renderer>,
        stage: RenderStage,
        jobs: &mut Vec<JobHandle>,
    ) {
        let renderer = Renderer::instance();

        let self_p = &raw const *self as u64;
        let pool = renderer.cmd_pool.clone();
        let job = SCHEDULER
            .job_builder("terrain_patches_render")
            .spawn(move || {
                let self_ref = unsafe { &*(self_p as *const Self) };
                let cmd = pool.get_command_list();
                if let Some(ao_vb) = renderer.ao_buffer.lock().as_ref().and_then(|h| h.get()) {
                    cmd.vertex_set_shader_resources(1, std::slice::from_ref(&ao_vb.srv.as_ref()));
                }
                self_ref.render(cmd, stage);
            });
        jobs.push(job);
    }

    fn subscribed_stages(&self) -> RenderStageSubscription {
        RenderStageSubscription::GENERATE_GBUFFER
            | RenderStageSubscription::SHADOW_GENERATE
            | RenderStageSubscription::DEPTH_PREPASS
    }
}
