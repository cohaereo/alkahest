use std::sync::Arc;

use alkahest_data::tfx::{
    common::AxisAlignedBBox,
    features::{decals::SDecalCollection, dynamic::RenderStageSubscription},
};

use super::FeatureRenderer;
use crate::{
    asset::{vertex_buffer::VertexBuffer, Handle},
    gpu::command_list::CommandList,
    tfx::technique::Technique,
    Renderer,
};

pub struct DecalCollectionRenderer {
    sets: Vec<DecalSet>,
    vb0: Handle<VertexBuffer>,
    vb1: Handle<VertexBuffer>,
    bounds: AxisAlignedBBox,
}

pub struct DecalSet {
    pub bounds: AxisAlignedBBox,
    /// Result of the occlusion test
    pub visible: bool,
    pub technique: Handle<Technique>,
    pub start: u16,
    pub count: u16,
}

impl DecalCollectionRenderer {
    #[profiling::function]
    pub fn load(collection: SDecalCollection) -> anyhow::Result<Box<Self>> {
        let vb0 = Renderer::instance().asset_manager.load(collection.vb0);
        let vb1 = Renderer::instance().asset_manager.load(collection.vb1);

        let sets = collection
            .decals
            .into_iter()
            .map(|set| {
                let r = (set.start as usize)..((set.start + set.count) as usize);
                DecalSet {
                    bounds: collection.decal_bounds.bounds[r]
                        .iter()
                        .map(|b| b.bb.clone())
                        .sum(),
                    visible: false,
                    technique: Renderer::instance().asset_manager.load(set.technique),
                    start: set.start,
                    count: set.count,
                }
            })
            .collect();

        Ok(Box::new(Self {
            sets,
            vb0,
            vb1,
            bounds: collection.bounds,
        }))
    }
}

impl FeatureRenderer for DecalCollectionRenderer {
    fn visibility_test(&mut self, camera: &crate::camera::Camera) -> bool {
        if !camera.culling_frustum.aabb_intersecting(&self.bounds) {
            return false;
        }

        let mut any_visible = false;
        for set in &mut self.sets {
            let is_visible = camera.culling_frustum.aabb_intersecting(&set.bounds);
            if is_visible {
                set.visible = true;
                any_visible = true;
            } else {
                set.visible = false;
            }
        }

        any_visible
    }

    fn extract_and_prepare(&mut self, _renderer: &Renderer, _extracted_data: &dyn std::any::Any) {}

    fn submit(&self, cmd: &mut CommandList, _stage: alkahest_data::tfx::RenderStage) {
        let Some((vb0, vb1)) = self.vb0.get().zip(self.vb1.get()) else {
            return;
        };

        Renderer::instance().globals.scopes.decal.bind(cmd).unwrap();

        cmd.input_assembler_set_vertex_buffers(
            0,
            &[Some(&vb1.buffer), Some(&vb0.buffer)],
            Some(&[vb1.stride, vb0.stride]),
            Some(&[0, 0]),
        )
        .unwrap();
        cmd.set_input_layout(17);
        cmd.set_input_topology(alkahest_data::tfx::PrimitiveType::Triangles);
        for set in self.sets.iter().filter(|s| s.visible) {
            let Some(t) = set.technique.get() else {
                continue;
            };
            t.bind(cmd).unwrap();
            cmd.draw_instanced(36_u32, set.count as u32, 0, set.start as u32);
        }
    }

    fn submit_parallel(
        &self,
        renderer: &Arc<Renderer>,
        _stage: alkahest_data::tfx::RenderStage,
        jobs: &mut Vec<alkahest_core::job::potassium::JobHandle>,
    ) {
        let renderer = renderer.clone();

        let self_p = &raw const *self as u64;
        let pool = renderer.cmd_pool.clone();
        // TODO(cohae): There's opportunity for optimization here. These jobs are currently quite coarse
        let job = alkahest_core::job::SCHEDULER
            .job_builder("decals_render")
            .spawn(move || {
                let self_ref = unsafe { &*(self_p as *const Self) };
                let cmd = pool.get_command_list();

                let Some((vb0, vb1)) = self_ref.vb0.get().zip(self_ref.vb1.get()) else {
                    return;
                };

                renderer.globals.scopes.decal.bind(cmd).unwrap();

                cmd.input_assembler_set_vertex_buffers(
                    0,
                    &[Some(&vb1.buffer), Some(&vb0.buffer)],
                    Some(&[vb1.stride, vb0.stride]),
                    Some(&[0, 0]),
                )
                .unwrap();
                cmd.set_input_layout(17);
                cmd.set_input_topology(alkahest_data::tfx::PrimitiveType::Triangles);
                for set in self_ref.sets.iter().filter(|s| s.visible) {
                    let Some(t) = set.technique.get() else {
                        continue;
                    };
                    t.bind(cmd).unwrap();
                    cmd.draw_instanced(36_u32, set.count as u32, 0, set.start as u32);
                }
            });
        jobs.push(job);
    }

    fn subscribed_stages(&self) -> RenderStageSubscription {
        RenderStageSubscription::DECALS
    }
}
