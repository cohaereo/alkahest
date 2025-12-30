use std::{f32, io::Write, ops::Deref, sync::Arc};

use alkahest_core::job::{
    potassium::{JobHandle, Priority},
    SCHEDULER,
};
use alkahest_data::tfx::{
    common::AxisAlignedBBox,
    features::{
        ao::SStaticAmbientOcclusion,
        dynamic::RenderStageSubscription,
        statics::{
            SStaticInstanceTransform, SStaticMesh, SStaticMeshInstances, SStaticSpecialMesh,
        },
    },
    RenderStage, ShaderStage,
};
use anyhow::Context;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};
use hashbrown::HashMap;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use tiger_parse::PackageManagerExt;
use tiger_pkg::{package_manager, TagHash};

use super::{shared::ModelBuffers, FeatureRenderer};
use crate::{
    asset::Handle,
    camera::Camera,
    gpu::{cbuffer::ConstantBuffer, command_list::CommandList},
    tfx::technique::Technique,
    Gpu, Renderer,
};

struct SpecialMesh {
    mesh: SStaticSpecialMesh,
    buffers: ModelBuffers,
    technique: Handle<Technique>,
}

impl Deref for SpecialMesh {
    type Target = SStaticSpecialMesh;

    fn deref(&self) -> &Self::Target {
        &self.mesh
    }
}

pub struct StaticModel {
    pub model: SStaticMesh,
    pub materials: Vec<Handle<Technique>>,
    pub hash: TagHash,
    pub subscribed_stages: RenderStageSubscription,
    buffers: Vec<ModelBuffers>,
    special_meshes: Vec<SpecialMesh>,
}

impl StaticModel {
    #[profiling::function]
    pub fn load(hash: TagHash) -> anyhow::Result<Self> {
        let model = package_manager().read_tag_struct::<SStaticMesh>(hash)?;
        let materials = model
            .techniques
            .iter()
            .map(|&tag| Renderer::instance().asset_manager.load::<Technique>(tag))
            .collect();

        let buffers = model
            .opaque_meshes
            .buffers
            .iter()
            .map(
                |&(index_buffer, vertex0_buffer, vertex1_buffer, color_buffer)| {
                    ModelBuffers::load(vertex0_buffer, vertex1_buffer, color_buffer, index_buffer)
                        .expect("Failed to load static model opaque mesh buffers")
                },
            )
            .collect();

        let mut subscribed_stages = model
            .opaque_meshes
            .mesh_groups
            .iter()
            .fold(RenderStageSubscription::empty(), |acc, group| {
                acc | group.render_stage
            });

        let special_meshes = model
            .special_meshes
            .iter()
            .map(|mesh| {
                subscribed_stages |= mesh.render_stage;
                SpecialMesh {
                    mesh: mesh.clone(),
                    buffers: ModelBuffers::load(
                        mesh.vertex0_buffer,
                        mesh.vertex1_buffer,
                        mesh.color_buffer,
                        mesh.index_buffer,
                    )
                    .expect("Failed to load special mesh buffers"),
                    technique: Renderer::instance().asset_manager.load(mesh.technique),
                }
            })
            .collect();

        Ok(Self {
            hash,
            model,
            materials,
            buffers,
            special_meshes,
            subscribed_stages,
        })
    }
}

pub struct StaticModelRenderer {
    instance_buffer: ConstantBuffer<u8>,
    model: StaticModel,
    instances: Vec<(SStaticInstanceTransform, AxisAlignedBBox)>,
    bounds: AxisAlignedBBox,
    identifier: u64,
    culling_enabled: bool,

    constants_dirty: bool,
    instance_count: u32,
}

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy)]
pub struct InstanceTransformBlock {
    pub transform: [Vec4; 3],
    pub params: Vec4,
}

impl StaticModelRenderer {
    pub fn new(
        gpu: &Arc<Gpu>,
        transforms: Vec<SStaticInstanceTransform>,
        occlusion_bounds: Option<Vec<AxisAlignedBBox>>,
        model_hash: TagHash,
        identifier: u64,
    ) -> anyhow::Result<Self> {
        let cbuffer = ConstantBuffer::create_raw(
            gpu,
            2 * size_of::<Vec4>() // quantization headers
            + transforms.len() * size_of::<InstanceTransformBlock>(), // per-transform data
        )?;

        let mut culling_enabled = true;
        let mut instances = Vec::with_capacity(transforms.len());
        for (i, transform) in transforms.into_iter().enumerate() {
            let bounds = if let Some(ref occlusion_bounds) = occlusion_bounds {
                occlusion_bounds[i].clone()
            } else {
                warn!(
                    "No occlusion bounds provided for static model instances \
                     (model_hash={model_hash}), culling disabled"
                );
                culling_enabled = false;
                AxisAlignedBBox::NONE
            };
            instances.push((transform, bounds));
        }

        trace!(instances = instances.len(), model_hash=%model_hash, "Loading model");
        Ok(Self {
            instance_buffer: cbuffer,
            model: StaticModel::load(model_hash)?,
            bounds: instances.iter().map(|(_, b)| b.clone()).sum(),
            culling_enabled,
            instance_count: instances.len() as u32,
            instances,
            identifier,
            constants_dirty: true,
        })
    }

    #[profiling::function]
    pub fn render_all(&self, cmd: &mut CommandList, stage: RenderStage) {
        let renderer = Renderer::instance();
        if let Some(ao_vb) = renderer.ao_buffer.lock().as_ref().and_then(|h| h.get()) {
            cmd.vertex_set_shader_resources(1, std::slice::from_ref(&ao_vb.srv.as_ref()));
        }

        self.instance_buffer
            .bind_cbuffer(cmd, ShaderStage::Vertex, 1);

        let is_opaque = matches!(
            stage,
            RenderStage::ShadowGenerate | RenderStage::DepthPrepass | RenderStage::GenerateGbuffer
        );

        if is_opaque {
            let opaque_meshes = &self.model.model.opaque_meshes;
            for (i, group, part) in opaque_meshes
                .mesh_groups
                .iter()
                .enumerate()
                .map(|(i, g)| (i, g, &opaque_meshes.parts[g.part_index as usize]))
                .filter(|(_, g, p)| g.render_stage == stage && p.lod_category.is_highest_detail())
            {
                let buffers = &self.model.buffers[part.buffer_index as usize];
                if buffers.bind(cmd).is_none() {
                    continue;
                }

                if let Some(technique) = &self.model.materials.get(i).and_then(Handle::get) {
                    technique.bind(cmd);
                } else {
                    continue;
                }

                cmd.set_input_layout(group.input_layout_index as usize);
                cmd.set_input_topology(part.primitive_type);

                cmd.draw_indexed_instanced(
                    part.index_count,
                    self.instance_count,
                    part.index_start,
                    0,
                    0,
                );
            }
        }

        if !is_opaque {
            for mesh in self
                .model
                .special_meshes
                .iter()
                .filter(|m| m.mesh.render_stage == stage && m.mesh.lod.is_highest_detail())
            {
                if mesh.buffers.bind(cmd).is_none() {
                    continue;
                }

                if let Some(technique) = &mesh.technique.get() {
                    technique.bind(cmd);
                } else {
                    continue;
                }
                cmd.set_input_layout(mesh.input_layout_index as usize);
                cmd.set_input_topology(mesh.primitive_type);

                cmd.draw_indexed_instanced(
                    mesh.index_count,
                    self.instance_count,
                    mesh.index_start,
                    0,
                    0,
                );
            }
        }
    }

    #[profiling::function]
    pub fn render_group(&self, cmd: &mut CommandList, stage: RenderStage, group: usize) {
        let renderer = Renderer::instance();
        if let Some(ao_vb) = renderer.ao_buffer.lock().as_ref().and_then(|h| h.get()) {
            cmd.vertex_set_shader_resources(1, std::slice::from_ref(&ao_vb.srv.as_ref()));
        }

        self.instance_buffer
            .bind_cbuffer(cmd, ShaderStage::Vertex, 1);

        let i = group;
        let group = &self.model.model.opaque_meshes.mesh_groups[i];
        if group.render_stage != stage {
            return;
        }
        let part = &self.model.model.opaque_meshes.parts[group.part_index as usize];
        if !part.lod_category.is_highest_detail() {
            return;
        }

        let buffers = &self.model.buffers[part.buffer_index as usize];
        if buffers.bind(cmd).is_none() {
            return;
        }

        if let Some(technique) = &self.model.materials.get(i).and_then(Handle::get) {
            technique.bind(cmd);
        } else {
            return;
        }

        cmd.set_input_layout(group.input_layout_index as usize);
        cmd.set_input_topology(part.primitive_type);

        cmd.draw_indexed_instanced(
            part.index_count,
            self.instance_count,
            part.index_start,
            0,
            0,
        );
    }

    #[profiling::function]
    pub fn update_constants(
        &self,
        ctx: &d3d11::DeviceContext,
        ao: Option<&SStaticAmbientOcclusion>,
    ) {
        let mut buffer = vec![];
        let model = &self.model.model.opaque_meshes;

        let vao_base = ao.and_then(|ao| ao.get_offset_by_identifier(self.identifier));

        buffer
            .write_all(bytemuck::cast_slice(&[
                model.mesh_offset.x,
                model.mesh_offset.y,
                model.mesh_offset.z,
                model.mesh_scale,
                model.texture_coordinate_scale,
                model.texture_coordinate_offset.x,
                model.texture_coordinate_offset.y,
                f32::from_bits(model.max_color_index),
            ]))
            .unwrap();

        // let model_transform = Mat4::from_cols_array_2d(&[
        //     [model.mesh_scale, 0.0, 0.0, model.mesh_offset.x],
        //     [0.0, model.mesh_scale, 0.0, model.mesh_offset.y],
        //     [0.0, 0.0, model.mesh_scale, model.mesh_offset.z],
        //     [0.0, 0.0, 0.0, 1.0],
        // ]);
        for (transform, _) in &self.instances {
            let instance_transform = Mat4::from_scale_rotation_translation(
                Vec3::splat(transform.scale),
                transform.rotation,
                transform.translation,
            )
            .transpose();
            // let instance_transform = Mat4::IDENTITY;

            let vertex_ao_offset = if let Some(vao_base) = vao_base {
                (transform.vertex_ao_offset + vao_base) >> 2
            } else {
                // println!("No AO for static model instance 0x{:016X}", self.identifier);
                0xFFFF_FFFF
            };

            buffer
                .write_all(bytemuck::cast_slice(&[InstanceTransformBlock {
                    transform: [
                        instance_transform.x_axis,
                        instance_transform.y_axis,
                        instance_transform.z_axis,
                    ],
                    params: Vec4::new(1.0, 1.0, 1.0, f32::from_bits(vertex_ao_offset)),
                }]))
                .unwrap();
        }

        unsafe {
            self.instance_buffer.write_array(ctx, &buffer).unwrap();
        }
    }

    fn visibility_test(&mut self, camera: &Camera) -> bool {
        if !self.culling_enabled {
            return false;
        }

        if !camera.culling_frustum.aabb_intersecting(&self.bounds) {
            return false;
        }

        true
    }
}

pub struct StaticInstancesRenderer {
    subscribed_stages: RenderStageSubscription,
    /// (model, visible)
    models: Vec<(StaticModelRenderer, bool)>,

    // (technique_hash, model_index, group_index) sorted by the group's technique hash
    groups_by_stage_sorted_by_technique: HashMap<RenderStage, Arc<Vec<(TagHash, usize, usize)>>>,
}

impl StaticInstancesRenderer {
    pub fn load(gpu: &Arc<Gpu>, instances_hash: TagHash) -> anyhow::Result<Self> {
        let instances: SStaticMeshInstances = package_manager().read_tag_struct(instances_hash)?;
        let mut models = Vec::with_capacity(instances.instance_groups.len());

        for group in &instances.instance_groups {
            let model = instances.statics[group.static_index as usize];
            let range = (group.instance_start as usize)
                ..(group.instance_start + group.instance_count) as usize;

            let renderer = StaticModelRenderer::new(
                gpu,
                instances
                    .transforms
                    .get(range.clone())
                    .context("Invalid instance transform range")?
                    .to_vec(),
                instances
                    .occlusion_bounds
                    .bounds
                    .get(range)
                    .map(|bounds| bounds.iter().map(|b| b.bb.clone()).collect()),
                model,
                instances.vertex_ao_identifier,
            )?;

            models.push((renderer, false));
        }

        let mut groups_by_stage_sorted_by_technique: HashMap<
            RenderStage,
            Vec<(TagHash, usize, usize)>,
        > = HashMap::default();
        for (model_index, (model, _visible)) in models.iter().enumerate() {
            for (group_index, (group, technique)) in model
                .model
                .model
                .opaque_meshes
                .mesh_groups
                .iter()
                .zip(model.model.materials.iter())
                .enumerate()
            {
                let part = &model.model.model.opaque_meshes.parts[group.part_index as usize];
                if part.lod_category.is_highest_detail() {
                    groups_by_stage_sorted_by_technique
                        .entry(group.render_stage)
                        .or_default()
                        .push((technique.hash(), model_index, group_index));
                }
            }
        }

        for (_stage, groups_sorted_by_technique) in groups_by_stage_sorted_by_technique.iter_mut() {
            groups_sorted_by_technique.sort_unstable_by_key(|k| k.0);
        }

        let groups_by_stage_sorted_by_technique = groups_by_stage_sorted_by_technique
            .into_iter()
            .map(|(k, v)| (k, Arc::new(v)))
            .collect();

        Ok(Self {
            subscribed_stages: models
                .iter()
                .fold(RenderStageSubscription::empty(), |acc, m| {
                    acc | m.0.model.subscribed_stages
                }),
            models,
            groups_by_stage_sorted_by_technique,
        })
    }
}

impl FeatureRenderer for StaticInstancesRenderer {
    fn visibility_test(&mut self, _camera: &Camera) -> bool {
        self.models.par_iter_mut().for_each(|(_model, visible)| {
            *visible = true; // model.visibility_test(camera);
        });
        true
    }

    fn extract_and_prepare(&mut self, renderer: &Renderer, _extracted_data: &dyn std::any::Any) {
        for (model, _visible) in self.models.iter_mut().filter(|(_, visible)| *visible) {
            if model.constants_dirty {
                model.update_constants(&renderer.gpu.context(), renderer.ao.read().as_ref());
                model.constants_dirty = false;
            }
        }
    }

    fn submit(&self, cmd: &mut CommandList, stage: RenderStage) {
        // Special meshes are rendered single-threaded for now
        for (model, _visible) in self.models.iter().filter(|(_, v)| *v) {
            model.render_all(cmd, stage);
        }
    }

    fn submit_parallel(
        &self,
        _renderer: &Arc<Renderer>,
        stage: RenderStage,
        jobs: &mut Vec<JobHandle>,
    ) {
        let Some(groups_sorted_by_technique) = self.groups_by_stage_sorted_by_technique.get(&stage)
        else {
            return;
        };

        let renderer = Renderer::instance();

        let node_count = groups_sorted_by_technique.len();
        // let nodes_per_job = node_count / job_count;
        // let mut last_end = 0;
        // let mut jobs_scheduled = 0;
        let mut schedule_range = |range: std::ops::Range<usize>| {
            let groups_sorted_by_technique = groups_sorted_by_technique.clone();
            let p_models = &self.models as *const _ as u64;
            let pool_clone = renderer.cmd_pool.clone();
            let job = SCHEDULER
                .job_builder("static_geometry")
                .priority(Priority::High)
                .spawn(move || {
                    let cmd_pooled = pool_clone.get_command_list();
                    // Safety: p_models is valid for the lifetime of this closure
                    // TODO(cohae): need a better way to pass self.models to the job
                    let p_models = p_models as *const Vec<(StaticModelRenderer, bool)>;
                    let models = unsafe { &*p_models };
                    for (_technique_hash, model_index, group_index) in
                        &groups_sorted_by_technique[range.clone()]
                    {
                        let (model, visible) = &models[*model_index];
                        if *visible {
                            model.render_group(cmd_pooled, stage, *group_index);
                        }
                    }
                });

            jobs.push(job);
        };

        let mut last_technique: Option<TagHash> = None;
        let mut last_range_start = 0;
        for (i, (technique, _model_index, _group_index)) in
            groups_sorted_by_technique.iter().enumerate()
        {
            if last_technique.is_none() {
                last_technique = Some(groups_sorted_by_technique[i].0);
            }

            if Some(*technique) != last_technique {
                let range = last_range_start..i;
                schedule_range(range.clone());
                last_technique = Some(*technique);
                last_range_start = i;
            }
        }

        if last_range_start < node_count {
            let range = last_range_start..node_count;
            schedule_range(range.clone());
        }

        // let initial_state = Arc::new(GpuState::backup(cmd));
        // let command_lists = job_ranges
        //     .par_iter()
        //     .map(|range| {
        //         let mut cmd = cmd.new_sublist();
        //         initial_state.restore(&mut cmd);
        //         for (_technique_hash, model_index, group_index) in
        //             &groups_sorted_by_technique[range.clone()]
        //         {
        //             let (model, visible) = &self.models[*model_index];
        //             if *visible {
        //                 model.render_group(&mut cmd, stage, *group_index);
        //             }
        //         }

        //         cmd
        //     })
        //     .collect::<Vec<_>>();
        // for command_list in command_lists {
        //     cmd.execute_command_list(&command_list.finish_command_list(false).unwrap(), true);
        // }
    }

    fn subscribed_stages(&self) -> RenderStageSubscription {
        self.subscribed_stages
    }
}

// impl FeatureRenderer for StaticModelRenderer {
//     fn visibility_test(&mut self, camera: &Camera) -> bool {
//         if !camera.culling_frustum.aabb_intersecting(&self.bounds) {
//             return false;
//         }

//         self.visible_instance_ids.clear();
//         for (i, (_, b)) in self.transforms.iter().enumerate() {
//             if camera.culling_frustum.aabb_intersecting(b) {
//                 self.visible_instance_ids.push(1 + i as u32);
//             }
//         }

//         !self.visible_instance_ids.is_empty()
//     }

//     fn extract_and_prepare(
//         &mut self,
//         renderer: &Renderer,
//         _data: &mut dyn super::FeatureRendererData,
//         _extracted_data: &dyn std::any::Any,
//     ) {
//         if self.constants_dirty {
//             self.update_constants(
//                 &renderer.gpu.context(), /*, renderer.ao.read().as_ref() */
//             );
//             self.constants_dirty = false;
//         }
//     }

//     fn submit(&self, cmd: &mut CommandList, stage: RenderStage) {
//         // Safety: there's never more instances than we allocated space for (hopefully)
//         unsafe {
//             self.instance_id_buffer
//                 .write(cmd, bytemuck::cast_slice(&self.visible_instance_ids))
//                 .unwrap();
//         }
//         self.render(cmd, stage);
//     }

//     fn subscribed_stages(&self) -> RenderStageSubscription {
//         self.model.subscribed_stages
//     }
// }
