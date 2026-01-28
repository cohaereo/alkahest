use alkahest_core::job::{SCHEDULER, potassium::Priority};
use alkahest_data::tfx::{
    RenderStage, ShaderStage,
    common::AxisAlignedBBox,
    features::{
        decorators::{SDecorator, SDecoratorInstanceElement},
        dynamic::RenderStageSubscription,
    },
};
use anyhow::Context;
use glam::{Mat4, Vec4, vec4};
use itertools::Itertools;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use tiger_pkg::TagHash;

use crate::{
    Renderer,
    asset::vertex_buffer::VertexBuffer,
    feature::{FeatureRenderer, rigid_model::DynamicModel},
    gpu::cbuffer::ConstantBuffer,
    tfx::externs,
    util::threading::CommandListSetId,
};

struct DecoratorModel {
    model: Box<DynamicModel>,
    ext: Box<externs::RigidModel>,
    cbuffers: Vec<ConstantBuffer<Vec4>>,
    identifier_mask: u32,
}

#[derive(Debug)]
struct DecoratorInstanceGroup {
    pub instance_start: u32,
    pub instance_count: u32,
    pub original_range: std::ops::Range<u32>,

    pub instance_bounds: Vec<(std::ops::Range<u32>, AxisAlignedBBox, bool)>,
    pub visible: bool,
    pub bounds: AxisAlignedBBox,
}

pub struct DecoratorRenderer {
    pub data: SDecorator,
    pub hash: TagHash,
    models: Vec<DecoratorModel>,
    // instance_buffer: Handle<VertexBuffer>,
    instance_buffer: VertexBuffer,
    instance_blend_indices_vb: VertexBuffer,

    instance_data_culled: Vec<SDecoratorInstanceElement>,
    instance_groups: Vec<DecoratorInstanceGroup>,
}

impl DecoratorRenderer {
    pub fn load(renderer: &Renderer, hash: TagHash, decorator: SDecorator) -> anyhow::Result<Self> {
        let mut models = vec![];
        for smodel in &decorator.unk8 {
            let model = DynamicModel::load(smodel.entity_model, vec![], vec![])?;
            let ext = Box::new(externs::RigidModel {
                local_to_world: Mat4::IDENTITY,
                position_scale: model.model.model_scale,
                position_offset: model.model.model_offset,
                texcoord0_scale_offset: Vec4::new(
                    model.model.texcoord_scale.x,
                    model.model.texcoord_scale.y,
                    model.model.texcoord_offset.x,
                    model.model.texcoord_offset.y,
                ),
                dynamic_sh_ao_values: Vec4::new(1.0, 1.0, 1.0, 0.0),
                ..Default::default()
            });

            let cbuffers = if let Some(unk34) = &*smodel.unk34 {
                let mut cbuffers = vec![];
                for constants in &unk34.unk8 {
                    let mut data = vec![Vec4::ONE; 72];
                    data[0..=4].copy_from_slice(&constants.unk0);
                    // data[5..=29]
                    for (i, c) in smodel.unk58.chunks_exact(4).enumerate() {
                        let v = Vec4::new(c[0], c[1], c[2], c[3]);
                        data[5 + i] = v;
                    }
                    // data[30..=33]
                    data[30] = Vec4::X;
                    data[31] = Vec4::Y;
                    data[32] = Vec4::Z;
                    data[33] = Vec4::W;

                    // data[34..=53]
                    let data_vec4 = [
                        vec4(91.46981, 0.0, 0.0, 0.0),
                        vec4(130.05363, 0.113788, 171.45092, 0.170682),
                        vec4(0.000810, 0.008098, 0.008098, 0.0),
                        vec4(0.046175, -2.079525, 0.0, 0.0),
                        vec4(1.125, 0.45, 1.125, 0.45),
                        vec4(0.433031, 0.433031, 0.0, 0.0),
                        vec4(0.0, 0.0, 0.0, 0.0),
                        vec4(88.568695, 0.138091, 1.0, 1.0),
                        vec4(63.486084, 0.017072, 0.202727, 1.25),
                        vec4(536.8253, 0.021957, 0.8, 0.0),
                        vec4(2.0, 0.7, 3.0, 0.3),
                        vec4(79.64881, 0.015265, 0.007953, 0.0),
                        vec4(0.019860, 0.264704, -224.83966, 0.0),
                        vec4(1144.3136, 0.172613, 0.5, 0.0),
                        vec4(1.2, 0.8, 0.3, 0.00005),
                        vec4(-89.79972, 0.060254, 0.120507, 0.004049),
                        vec4(0.120507, 2744.3176, -2.489936, 0.0),
                        vec4(1.5, 1.5, 0.45, 0.5),
                        vec4(1.0, 1.0, 0.0, 0.0),
                        vec4(-8.742278e-08, -1.0, 0.0, 0.133937),
                    ];
                    for (i, v) in data_vec4.iter().enumerate() {
                        data[34 + i] = *v;
                    }

                    cbuffers.push(ConstantBuffer::create_array(
                        &renderer.gpu,
                        data.len(),
                        Some(&data),
                    )?)
                }

                cbuffers
            } else {
                vec![]
            };

            let identifier_mask = if decorator.unk8.len() <= 1 {
                u32::MAX
            } else {
                model.model.meshes[0..model.model.meshes.len() - 1]
                    .iter()
                    .fold(0, |acc, mesh| {
                        let first_id = mesh
                            .parts
                            .first()
                            .map(|p| p.external_identifier)
                            .unwrap_or(0);
                        if first_id > 31 {
                            acc
                        } else {
                            acc | 1 << first_id
                        }
                    })
            };

            models.push(DecoratorModel {
                model,
                ext,
                cbuffers,
                identifier_mask,
            });
        }

        if models.is_empty() {
            anyhow::bail!("No models found in decorator");
        }

        // u8 for decorators, f32 for speedtree
        // let blend_index_data = vec![0xC8u8; decorator.unk48.instance_data.data.len() * 4];
        let blend_index_data = vec![1f32; decorator.unk48.instance_data.elements.len() * 4];
        let instance_blend_indices_vb =
            VertexBuffer::load_data(&renderer.gpu, bytemuck::cast_slice(&blend_index_data), 4)?;

        let mut instance_range_bounds = Vec::with_capacity(decorator.unk38.len());
        for (i, &[instance_start, instance_end]) in decorator.unk38.array_windows::<2>().enumerate()
        {
            let bounds_index = decorator
                .unk38_to_bounds_index
                .get(i)
                .copied()
                .context("Invalid bounds index mapping")?;

            let group_bounds = decorator
                .occlusion_bounds
                .bounds
                .get(bounds_index as usize)
                .context("Invalid bounds index")?
                .bb;

            instance_range_bounds.push((instance_start..instance_end, group_bounds));
        }

        let mut instance_groups = vec![];
        for &[instance_start, instance_end] in decorator.unk18.array_windows::<2>() {
            let bounds_start = instance_range_bounds
                .iter()
                .position(|(range, _)| range.start == instance_start)
                .context("Failed to find occlusion bounds start")?;
            let bounds_end = instance_range_bounds
                .iter()
                .position(|(range, _)| range.end == instance_end)
                .context("Failed to find occlusion bounds end")?;
            let bounds_range = bounds_start..=bounds_end;
            let instance_bounds = instance_range_bounds
                .get(bounds_range.clone())
                .context("Failed to find occlusion bounds")?
                .iter()
                .map(|(range, bb)| (range.clone(), *bb, true))
                .collect_vec();

            let bounds = instance_bounds.iter().map(|(_, bb, _)| *bb).sum();

            instance_groups.push(DecoratorInstanceGroup {
                original_range: instance_start..instance_end,
                instance_start,
                instance_count: instance_end - instance_start,
                visible: true,
                instance_bounds,
                bounds,
            });
        }

        let instance_buffer = VertexBuffer::load_data_ex(
            &renderer.gpu,
            bytemuck::cast_slice(&decorator.unk48.instance_data.elements),
            16,
            true,
        )?;

        Ok(Self {
            models,
            hash,
            instance_buffer,
            instance_blend_indices_vb,
            instance_data_culled: decorator.unk48.instance_data.elements.clone(),
            data: decorator,
            instance_groups,
        })
    }
}

#[profiling::all_functions]
impl FeatureRenderer for DecoratorRenderer {
    fn visibility_test(&mut self, camera: &crate::camera::Camera) -> bool {
        if !camera.is_visible(&self.data.bounds) {
            return false;
        }

        self.instance_groups.par_iter_mut().for_each(|group| {
            group.visible = camera.is_visible(&group.bounds);
            // if !group.visible {
            //     Renderer::instance().immediate.lock().aabb_world(
            //         &group.bounds,
            //         if group.visible { 0x00ff00 } else { 0xff0000 },
            //     );
            // }
            if group.visible {
                group
                    .instance_bounds
                    .par_iter_mut()
                    .for_each(|(_range, bounds, visible)| {
                        *visible = camera.is_visible(bounds);
                        // if !*visible {
                        //     Renderer::instance()
                        //         .immediate
                        //         .lock()
                        //         .aabb_world(bounds, if *visible { 0x00ff00 } else { 0xff0000 });
                        // }
                    });

                group.visible = group.instance_bounds.is_empty()
                    || group.instance_bounds.iter().any(|(_, _, vis)| *vis);
            }
        });

        self.instance_groups.iter().any(|g| g.visible)
    }

    fn extract_and_prepare(&mut self, renderer: &Renderer, extracted_data: &dyn std::any::Any) {
        _ = renderer;
        _ = extracted_data;

        {
            let mut instance_data_culled = vec![];
            std::mem::swap(&mut self.instance_data_culled, &mut instance_data_culled);
            instance_data_culled.clear();
            for group in self.instance_groups.iter_mut().filter(|g| g.visible) {
                group.instance_start = instance_data_culled.len() as u32;
                group.instance_count = 0;
                for (instance_range, _bounds, _visible) in
                    group.instance_bounds.iter().filter(|(_, _, vis)| *vis)
                {
                    for instance in instance_range.clone() {
                        instance_data_culled
                            .push(self.data.unk48.instance_data.elements[instance as usize]);
                    }
                    group.instance_count += instance_range.len() as u32;
                }
            }
            std::mem::swap(&mut instance_data_culled, &mut self.instance_data_culled);
        }

        unsafe {
            self.instance_buffer.write(
                &renderer.gpu.context(),
                bytemuck::cast_slice(&self.instance_data_culled),
            )
        }
        .expect("Failed to write decorator instance")
    }

    fn submit(
        &self,
        cmd: &mut crate::gpu::command_list::CommandList,
        stage: alkahest_data::tfx::RenderStage,
    ) {
        // cmd_event_span!(cmd, format!("<decorator {} ({} models)>", self.hash, self.models.len()));

        let renderer = Renderer::instance();
        let speedtree_vertex_slot = renderer.globals.scopes.speedtree.vertex_slot() as u32;

        {
            let consts = &self.data.unk48.unk14;
            cmd.externs.speedtree_placements = Box::new(externs::SpeedtreePlacements {
                unk10: Vec4::W,
                unk20: consts.instances_scale,
                unk30: consts.instances_offset,
                unk40: consts.unk20,
                unk50: consts.unk30,
                unk60: consts.unk40,
                unk70: consts.unk50,
                ..(cmd
                    .externs
                    .speedtree_placements
                    .as_deref()
                    .cloned()
                    .unwrap_or_default())
            })
            .into();
        }

        for (id, group) in self
            .instance_groups
            .iter()
            .enumerate()
            .filter(|(_, g)| g.visible)
        {
            // let group_mask = if self.models.len() == 1 {
            //     1 << id
            // } else {
            //     // cohae: Multi-models (usually trees) seem to use the ID as a LOD level?
            //     u64::MAX
            // };

            let model_id = if self.models.len() == 1 { 0 } else { id };
            // cmd_event_span!(cmd, format!("<id {}, model {}>", id, model_id));

            let Some(model) = self.models.get(model_id) else {
                warn!(
                    decorator_set = %self.hash,
                    mesh_count = self.models[0].model.model.meshes.len(),
                    variant_count = self.models[0].model.variant_count(),
                    identifier_count = self.models[0].model.identifier_count(),
                    "Decorator model index {id} out of bounds for models list of length {}",
                    self.models.len(),
                );
                continue;
            };

            cmd.externs.rigid_model = model.ext.clone().into();

            let identifier_mask = if self.models.len() == 1 {
                1u32.unbounded_shl(id as u32)
            } else {
                // cohae: There's dedicated identifiers for the parts used for ShadowGenerate, just doing +1 (or << 1) seems to work?
                match stage {
                    RenderStage::ShadowGenerate => model.identifier_mask << 1,
                    _ => model.identifier_mask,
                }
            };

            self.instance_blend_indices_vb.bind_single(cmd, 3);
            model.model.draw_wrapped(
                cmd,
                stage,
                identifier_mask,
                move |_model, cmd, (mesh_index, _mesh), (_part_index, part)| {
                    if let Some(cb) = model.cbuffers.get(mesh_index) {
                        cb.bind(cmd, ShaderStage::Vertex, speedtree_vertex_slot);
                    } else {
                        cmd.vertex_set_constant_buffers(speedtree_vertex_slot, &[None]);
                    }

                    self.instance_buffer.bind_single(cmd, 1);

                    cmd.draw_indexed_instanced(
                        part.index_count,
                        group.instance_count,
                        part.index_start,
                        0,
                        group.instance_start,
                    );
                },
            );
        }
    }

    fn submit_parallel(
        &self,
        renderer: &std::sync::Arc<Renderer>,
        set: CommandListSetId,
        stage: alkahest_data::tfx::RenderStage,
        jobs: &mut Vec<alkahest_core::job::potassium::JobHandle>,
    ) {
        let self_p = (&raw const *self) as u64;
        let pool_clone = renderer.cmd_pool.clone();
        let job = SCHEDULER
            .job_builder("decorators_render")
            .priority(Priority::High)
            .spawn(move || {
                let self_ref = unsafe { &*(self_p as *const DecoratorRenderer) };
                let cmd = pool_clone.get_command_list(set);
                self_ref.submit(cmd, stage);
            });

        jobs.push(job);
    }

    fn subscribed_stages(&self) -> RenderStageSubscription {
        self.models
            .iter()
            .fold(RenderStageSubscription::empty(), |acc, model| {
                acc | model.model.subscribed_stages
            })
    }
}
