use alkahest_core::job::{potassium::Priority, SCHEDULER};
use alkahest_data::tfx::{
    features::{decorators::SDecorator, dynamic::RenderStageSubscription},
    ShaderStage,
};
use glam::{Mat4, Vec4};
use tiger_pkg::TagHash;

use crate::{
    asset::{vertex_buffer::VertexBuffer, Handle},
    feature::{rigid_model::DynamicModel, FeatureRenderer},
    gpu::cbuffer::ConstantBuffer,
    tfx::externs,
    util::threading::CommandListSetId,
    Renderer,
};

pub struct DecoratorRenderer {
    pub data: SDecorator,
    pub hash: TagHash,
    #[allow(clippy::type_complexity)]
    pub models: Vec<(
        Box<DynamicModel>,
        Box<externs::RigidModel>,
        Option<ConstantBuffer<Vec4>>,
    )>,
    instance_buffer: Handle<VertexBuffer>,
    instance_blend_indices_vb: VertexBuffer,
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

            let speedtree_cbuffer = if let Some(unk34) = &*smodel.unk34 {
                let mut data = vec![Vec4::ONE; 72];
                data[0..=4].copy_from_slice(&unk34.unk8.get(1).unwrap_or(&unk34.unk8[0]).unk0);
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
                let ndata: [u8; 0x140] = [
                    0x8B, 0xF0, 0xB6, 0x42, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0xBB, 0x0D, 0x02, 0x43, 0x9E, 0x09, 0xE9, 0x3D, 0x70, 0x73,
                    0x2B, 0x43, 0x36, 0xC7, 0x2E, 0x3E, 0x9E, 0x45, 0x54, 0x3A, 0x83, 0xAB, 0x04,
                    0x3C, 0x83, 0xAB, 0x04, 0x3C, 0x00, 0x00, 0x00, 0x00, 0xD0, 0x21, 0x3D, 0x3D,
                    0xEE, 0x16, 0x05, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x90, 0x3F, 0x67, 0x66, 0xE6, 0x3E, 0x00, 0x00, 0x90, 0x3F, 0x67, 0x66,
                    0xE6, 0x3E, 0x47, 0xB6, 0xDD, 0x3E, 0x47, 0xB6, 0xDD, 0x3E, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2C, 0x23, 0xB1, 0x42, 0xCD,
                    0x67, 0x0D, 0x3E, 0x00, 0x00, 0x80, 0x3F, 0x00, 0x00, 0x80, 0x3F, 0xC0, 0xF1,
                    0x7D, 0x42, 0x92, 0xDB, 0x8B, 0x3C, 0xA5, 0x97, 0x4F, 0x3E, 0x00, 0x00, 0xA0,
                    0x3F, 0xD2, 0x34, 0x06, 0x44, 0xBE, 0xDE, 0xB3, 0x3C, 0xCD, 0xCC, 0x4C, 0x3F,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x33, 0x33, 0x33, 0x3F, 0x00,
                    0x00, 0x40, 0x40, 0x9A, 0x99, 0x99, 0x3E, 0x31, 0x4C, 0x9F, 0x42, 0x0D, 0x1A,
                    0x7A, 0x3C, 0x0B, 0x4E, 0x02, 0x3C, 0x00, 0x00, 0x00, 0x00, 0xCA, 0xB0, 0xA2,
                    0x3C, 0x40, 0x87, 0x87, 0x3E, 0xF4, 0xD6, 0x60, 0xC3, 0x00, 0x00, 0x00, 0x00,
                    0x09, 0x0A, 0x8F, 0x44, 0x94, 0xC1, 0x30, 0x3E, 0x00, 0x00, 0x00, 0x3F, 0x00,
                    0x00, 0x00, 0x00, 0x9A, 0x99, 0x99, 0x3F, 0xCD, 0xCC, 0x4C, 0x3F, 0x9A, 0x99,
                    0x99, 0x3E, 0x18, 0xB7, 0x51, 0x38, 0x76, 0x99, 0xB3, 0xC2, 0x82, 0xCC, 0x76,
                    0x3D, 0x72, 0xCC, 0xF6, 0x3D, 0x99, 0xAB, 0x84, 0x3B, 0x72, 0xCC, 0xF6, 0x3D,
                    0x15, 0x85, 0x2B, 0x45, 0x1D, 0x5B, 0x1F, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0xC0, 0x3F, 0x00, 0x00, 0xC0, 0x3F, 0x67, 0x66, 0xE6, 0x3E, 0x00, 0x00,
                    0x00, 0x3F, 0x00, 0x00, 0x80, 0x3F, 0x00, 0x00, 0x80, 0x3F, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x2E, 0xBD, 0xBB, 0xB3, 0x00, 0x00, 0x80, 0xBF,
                    0x00, 0x00, 0x00, 0x00, 0xBA, 0x26, 0x09, 0x3E,
                ];
                // data[34..=53]
                // copy raw data
                let data_vec4: &[[f32; 4]] = bytemuck::cast_slice(&ndata);
                for (i, v) in data_vec4.iter().enumerate() {
                    data[34 + i] = Vec4::from_array(*v);
                }
                Some(ConstantBuffer::create_array(
                    &renderer.gpu,
                    data.len(),
                    Some(&data),
                )?)
            } else {
                None
            };

            models.push((model, ext, speedtree_cbuffer));
        }

        if models.is_empty() {
            anyhow::bail!("No models found in decorator");
        }

        if models.len() > 1 {
            // anyhow::bail!("Decorators with more than one model are not supported yet");
            warn!("Decorators with more than one model are WIP");
        }

        // u8 for decorators, f32 for speedtree
        // let blend_index_data = vec![0xC8u8; decorator.unk48.instance_data.data.len() * 4];
        let blend_index_data = vec![1f32; decorator.unk48.instance_data.data.len() * 4];
        let instance_blend_indices_vb =
            VertexBuffer::load_data(&renderer.gpu, bytemuck::cast_slice(&blend_index_data), 4)?;

        Ok(Self {
            models,
            hash,
            instance_buffer: renderer.asset_manager.load(decorator.unk48.instance_buffer),
            instance_blend_indices_vb,
            data: decorator,
        })
    }
}

impl FeatureRenderer for DecoratorRenderer {
    fn visibility_test(&mut self, camera: &crate::camera::Camera) -> bool {
        camera.culling_frustum.aabb_intersecting(&self.data.bounds)
    }

    fn extract_and_prepare(&mut self, renderer: &Renderer, extracted_data: &dyn std::any::Any) {
        _ = renderer;
        _ = extracted_data;

        // for (m, _, _) in &mut self.models {
        //     m.update_cbuffer(&renderer.gpu, Mat4::IDENTITY, None)
        //         .expect("Failed to update cbuffer for decorator model");
        // }
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
                // unk40: Default::default(),
                // unk50: Default::default(),
                // unk60: Default::default(),
                ..(cmd
                    .externs
                    .speedtree_placements
                    .as_deref()
                    .cloned()
                    .unwrap_or_default())
            })
            .into();
        }

        for id in 0..(self.data.unk18.len() - 1) {
            let instance_start = self.data.unk18[id];
            let instance_end = self.data.unk18[id + 1];
            let instance_count = instance_end - instance_start;

            // let group_mask = if self.models.len() == 1 {
            //     1 << id
            // } else {
            //     // cohae: Multi-models (usually trees) seem to use the ID as a LOD level?
            //     u64::MAX
            // };

            let model_id = if self.models.len() == 1 { 0 } else { id };
            // cmd_event_span!(cmd, format!("<id {}, model {}>", id, model_id));

            let Some((model, ext, cb)) = self.models.get(model_id) else {
                warn!(
                    decorator_set = %self.hash,
                    mesh_count = self.models[0].0.model.meshes.len(),
                    variant_count = self.models[0].0.variant_count(),
                    identifier_count = self.models[0].0.identifier_count(),
                    "Decorator model index {id} out of bounds for models list of length {}",
                    self.models.len(),
                );
                continue;
            };

            if let Some(cb) = cb {
                cb.bind(cmd, ShaderStage::Vertex, speedtree_vertex_slot);
            } else {
                cmd.vertex_set_constant_buffers(speedtree_vertex_slot, &[None]);
            }

            cmd.externs.rigid_model = Box::clone(ext).into();

            let dyn_id = if self.models.len() == 1 {
                id as u16
            } else {
                // cohae: Multi-models (usually trees) seem to use the ID as a LOD level?
                0
            };

            // println!(
            //     "Drawing ID {id} (max {}, {} identifiers for model {} in set {})",
            //     self.models.len(),
            //     model.identifier_count(),
            //     model_index,
            //     self.hash
            // );

            self.instance_blend_indices_vb.bind_single(cmd, 3);
            model.draw_wrapped(cmd, stage, dyn_id, move |_model, cmd, _mesh, part| {
                // Seems to be the LOD selector. 0=high, 1=medium, 2=low
                // if part.unk17 != 2 {
                //     return;
                // }

                let Some(cb) = self.instance_buffer.get() else {
                    return;
                };
                cb.bind_single(cmd, 1);

                cmd.draw_indexed_instanced(
                    part.index_count,
                    // self.data.unk48.instance_data.data.len() as _,
                    // instance_count.min(4096),
                    instance_count,
                    part.index_start,
                    0,
                    instance_start,
                );
            });
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
            .fold(RenderStageSubscription::empty(), |acc, (model, _, _)| {
                acc | model.subscribed_stages
            })
    }
}
