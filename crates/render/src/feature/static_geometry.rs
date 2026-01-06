use std::{f32, io::Write, ops::Deref, sync::Arc};

use ahash::{HashMap, HashSet};
use alkahest_core::job::{
    SCHEDULER,
    potassium::{JobHandle, Priority},
};
use alkahest_data::tfx::{
    RenderStage, ShaderStage,
    common::AxisAlignedBBox,
    features::{
        ao::SStaticAmbientOcclusion,
        dynamic::RenderStageSubscription,
        statics::{
            SStaticInstanceTransform, SStaticMesh, SStaticMeshInstances, SStaticSpecialMesh,
        },
    },
};
use anyhow::Context;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use smallvec::SmallVec;
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

use super::{FeatureRenderer, shared::ModelBuffers};
use crate::{
    Gpu, Renderer,
    asset::Handle,
    camera::Camera,
    gpu::{cbuffer::ConstantBuffer, command_list::CommandList},
    tfx::technique::Technique,
    util::threading::CommandListSetId,
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

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy)]
pub struct InstanceTransformBlock {
    pub transform: [Vec4; 3],
    pub params: Vec4,
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

    #[profiling::function]
    pub fn render_all(&self, cmd: &mut CommandList, stage: RenderStage, instance_count: u32) {
        if !self.subscribed_stages.is_subscribed(stage) {
            return;
        }

        let renderer = Renderer::instance();
        if let Some(ao_vb) = renderer.ao_buffer.read().as_ref().and_then(|h| h.get()) {
            cmd.vertex_set_shader_resources(1, std::slice::from_ref(&ao_vb.srv.as_ref()));
        }

        // self.instance_buffer
        //     .bind_cbuffer(cmd, ShaderStage::Vertex, 1);

        let is_opaque = matches!(
            stage,
            RenderStage::ShadowGenerate | RenderStage::DepthPrepass | RenderStage::GenerateGbuffer
        );

        if is_opaque {
            let opaque_meshes = &self.model.opaque_meshes;
            for (i, group, part) in opaque_meshes
                .mesh_groups
                .iter()
                .enumerate()
                .map(|(i, g)| (i, g, &opaque_meshes.parts[g.part_index as usize]))
                .filter(|(_, g, p)| g.render_stage == stage && p.lod_category.is_highest_detail())
            {
                let buffers = &self.buffers[part.buffer_index as usize];
                if buffers.bind(cmd).is_none() {
                    continue;
                }

                if let Some(technique) = &self.materials.get(i).and_then(Handle::get) {
                    technique.bind(cmd);
                } else {
                    continue;
                }

                cmd.set_input_layout(group.input_layout_index as usize);
                cmd.set_input_topology(part.primitive_type);

                cmd.draw_indexed_instanced(
                    part.index_count,
                    instance_count,
                    part.index_start,
                    0,
                    0,
                );
            }
        }

        if !is_opaque {
            for mesh in self
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
                    instance_count,
                    mesh.index_start,
                    0,
                    0,
                );
            }
        }
    }

    #[profiling::function]
    pub fn render_group(
        &self,
        cmd: &mut CommandList,
        stage: RenderStage,
        instance_count: u32,
        group: usize,
    ) {
        let i = group;
        let group = &self.model.opaque_meshes.mesh_groups[i];
        if group.render_stage != stage {
            return;
        }

        let part = &self.model.opaque_meshes.parts[group.part_index as usize];
        if !part.lod_category.is_highest_detail() {
            return;
        }

        let renderer = Renderer::instance();
        if let Some(ao_vb) = renderer.ao_buffer.read().as_ref().and_then(|h| h.get()) {
            cmd.vertex_set_shader_resources(1, std::slice::from_ref(&ao_vb.srv.as_ref()));
        }

        let buffers = &self.buffers[part.buffer_index as usize];
        if buffers.bind(cmd).is_none() {
            return;
        }

        if let Some(technique) = &self.materials.get(i).and_then(Handle::get) {
            cmd.enable_smart_technique_binding();
            technique.bind(cmd);
        } else {
            return;
        }

        cmd.set_input_layout(group.input_layout_index as usize);
        cmd.set_input_topology(part.primitive_type);

        cmd.draw_indexed_instanced(part.index_count, instance_count, part.index_start, 0, 0);
    }
}

struct StaticInstanceGroup {
    pub transforms: Vec<SStaticInstanceTransform>,
    pub static_index: u16,
    pub cbuffer: ConstantBuffer<u8>,
    pub bounds: Vec<AxisAlignedBBox>,
    pub group_bounds: AxisAlignedBBox,
    pub visible: bool,
}

impl StaticInstanceGroup {
    #[profiling::function]
    pub fn update_constants(
        &self,
        ctx: &d3d11::DeviceContext,
        model: &StaticModel,
        vao_identifier: u64,
        ao: Option<&SStaticAmbientOcclusion>,
    ) {
        let mut data = vec![];
        let model = &model.model.opaque_meshes;

        let vao_base = ao.and_then(|ao| ao.get_offset_by_identifier(vao_identifier));

        data.write_all(bytemuck::cast_slice(&[
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
        for transform in &self.transforms {
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

            data.write_all(bytemuck::cast_slice(&[InstanceTransformBlock {
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
            self.cbuffer.write_array(ctx, &data).unwrap();
        }
    }
}

pub struct StaticInstancesRenderer {
    subscribed_stages: RenderStageSubscription,

    static_models: Vec<StaticModel>,
    groups: Vec<StaticInstanceGroup>,

    vao_identifier: u64,
    constants_dirty: bool,
    groups_by_stage_sorted_by_technique: HashMap<RenderStage, Arc<Vec<SortedModel>>>,
    bounds: AxisAlignedBBox,
}

impl StaticInstancesRenderer {
    pub fn load(gpu: &Arc<Gpu>, instances_hash: TagHash) -> anyhow::Result<Self> {
        let instances: SStaticMeshInstances = package_manager().read_tag_struct(instances_hash)?;
        let mut static_models = Vec::with_capacity(instances.statics.len());

        for model in &instances.statics {
            let renderer = StaticModel::load(*model)?;

            static_models.push(renderer);
        }

        let mut model_to_instance_groups: HashMap<u16, SmallVec<[usize; 4]>> = HashMap::default();
        let mut groups = Vec::with_capacity(instances.instance_groups.len());
        for (i, group) in instances.instance_groups.iter().enumerate() {
            let range = (group.instance_start as usize)
                ..(group.instance_start + group.instance_count) as usize;

            let transforms = instances
                .transforms
                .get(range.clone())
                .context("Invalid instance transform range")?
                .to_vec();

            let mut bounds = Vec::with_capacity(transforms.len());
            for i in range {
                let bounds_index = if instances.transform_to_bounds_index.is_empty() {
                    i
                } else {
                    *instances
                        .transform_to_bounds_index
                        .get(i)
                        .context("Invalid transform to bounds index")? as usize
                };
                let b = instances
                    .occlusion_bounds
                    .bounds
                    .get(bounds_index)
                    .context("Invalid occlusion bounds index")?;
                bounds.push(b.bb.clone());
            }

            let group_bounds = bounds.iter().cloned().sum();

            let cbuffer = ConstantBuffer::create_raw(
                gpu,
                2 * size_of::<Vec4>() // quantization headers
                            + transforms.len() * size_of::<InstanceTransformBlock>(), // per-transform data
            )?;

            groups.push(StaticInstanceGroup {
                transforms,
                bounds,
                group_bounds,
                static_index: group.static_index,
                cbuffer,
                visible: true,
            });
            model_to_instance_groups
                .entry(group.static_index)
                .or_default()
                .push(i);
        }

        let mut groups_by_stage_sorted_by_technique: HashMap<RenderStage, Vec<SortedModel>> =
            HashMap::default();
        for (model_index, model) in static_models.iter().enumerate() {
            for (group_index, (group, technique)) in model
                .model
                .opaque_meshes
                .mesh_groups
                .iter()
                .zip(model.materials.iter())
                .enumerate()
            {
                let part = &model.model.opaque_meshes.parts[group.part_index as usize];
                if part.lod_category.is_highest_detail() {
                    groups_by_stage_sorted_by_technique
                        .entry(group.render_stage)
                        .or_default()
                        .push(SortedModel {
                            technique: technique.hash(),
                            model_index,
                            group_index,
                            instance_groups: model_to_instance_groups
                                .get(&(model_index as u16))
                                .cloned()
                                .unwrap_or_default(),
                        });
                }
            }
        }

        for (_stage, groups_sorted_by_technique) in groups_by_stage_sorted_by_technique.iter_mut() {
            groups_sorted_by_technique.sort_unstable_by_key(|k| k.technique);
        }

        let groups_by_stage_sorted_by_technique: HashMap<RenderStage, Arc<Vec<SortedModel>>> =
            groups_by_stage_sorted_by_technique
                .into_iter()
                .map(|(k, v)| (k, Arc::new(v)))
                .collect();

        Ok(Self {
            subscribed_stages: static_models
                .iter()
                .fold(RenderStageSubscription::empty(), |acc, m| {
                    acc | m.subscribed_stages
                }),
            static_models,
            groups,
            constants_dirty: true,
            vao_identifier: instances.vertex_ao_identifier,
            groups_by_stage_sorted_by_technique,
            bounds: instances.bounds,
        })
    }
}

impl FeatureRenderer for StaticInstancesRenderer {
    fn visibility_test(&mut self, camera: &Camera) -> bool {
        if !camera.is_visible(&self.bounds) {
            return false;
        }

        self.groups.par_iter_mut().for_each(|group| {
            group.visible = camera.is_visible(&group.group_bounds);
            if group.visible {
                group.visible = group.bounds.iter().any(|b| camera.is_visible(b));
            }
        });

        self.groups.iter().any(|m| m.visible)
    }

    fn extract_and_prepare(&mut self, renderer: &Renderer, _extracted_data: &dyn std::any::Any) {
        if self.constants_dirty {
            for group in &self.groups {
                let model = &self.static_models[group.static_index as usize];
                group.update_constants(
                    &renderer.gpu.context(),
                    model,
                    self.vao_identifier,
                    renderer.ao.read().as_ref(),
                );
            }
            self.constants_dirty = false;
        }
    }

    fn submit(&self, cmd: &mut CommandList, stage: RenderStage) {
        for group in self.groups.iter().filter(|g| {
            let model = &self.static_models[g.static_index as usize];

            // TODO(cohae): This is a hack for getting shadow generation to work, since shadow mapped lights dont have their own views yet, and thus can't cull statics
            let visible = if stage == RenderStage::ShadowGenerate {
                true
            } else {
                g.visible
            };

            visible && model.subscribed_stages.is_subscribed(stage)
        }) {
            let model = &self.static_models[group.static_index as usize];
            group.cbuffer.bind_cbuffer(cmd, ShaderStage::Vertex, 1);
            model.render_all(cmd, stage, group.transforms.len() as u32);
        }
    }

    fn submit_parallel(
        &self,
        _renderer: &Arc<Renderer>,
        set: CommandListSetId,
        stage: RenderStage,
        jobs: &mut Vec<JobHandle>,
    ) {
        let renderer = Renderer::instance();

        let Some(groups_sorted_by_technique) = self.groups_by_stage_sorted_by_technique.get(&stage)
        else {
            for (i, _group) in self.groups.iter().enumerate().filter(|(_i, g)| {
                let model = &self.static_models[g.static_index as usize];
                g.visible && model.subscribed_stages.is_subscribed(stage)
            }) {
                let p_models = &self.static_models as *const _ as u64;
                let p_groups = &self.groups as *const _ as u64;
                let pool_clone = renderer.cmd_pool.clone();
                let job = SCHEDULER
                    .job_builder("static_geometry")
                    .priority(Priority::High)
                    .spawn(move || {
                        let cmd = pool_clone.get_command_list(set);
                        // Safety: p_models/p_groups are (practically) valid for the lifetime of this closure
                        // TODO(cohae): need a safer way to pass self.models to the job
                        let p_models = p_models as *const Vec<StaticModel>;
                        let models = unsafe { &*p_models };
                        let p_groups = p_groups as *const Vec<StaticInstanceGroup>;
                        let groups = unsafe { &*p_groups };
                        let group = &groups[i];
                        let model = &models[group.static_index as usize];
                        group.cbuffer.bind_cbuffer(cmd, ShaderStage::Vertex, 1);
                        model.render_all(cmd, stage, group.transforms.len() as u32);
                    });

                jobs.push(job);
            }
            return;
        };

        let node_count = groups_sorted_by_technique.len();
        // let nodes_per_job = node_count / job_count;
        // let mut last_end = 0;
        // let mut jobs_scheduled = 0;
        let mut schedule_range = |range: std::ops::Range<usize>| {
            let groups_sorted_by_technique = groups_sorted_by_technique.clone();
            let p_models = &self.static_models as *const _ as u64;
            let p_groups = &self.groups as *const _ as u64;
            let pool_clone = renderer.cmd_pool.clone();

            let visible = groups_sorted_by_technique[range.clone()].iter().any(|r| {
                let group_indices = &r.instance_groups;
                group_indices.iter().any(|&gi| self.groups[gi].visible)
            });

            if !visible {
                return;
            }

            let job = SCHEDULER
                .job_builder("static_geometry")
                .priority(Priority::High)
                .spawn(move || {
                    let cmd = pool_clone.get_command_list(set);
                    // Safety: p_models/p_groups are (practically) valid for the lifetime of this closure
                    // TODO(cohae): need a safer way to pass self.models to the job
                    let p_models = p_models as *const Vec<StaticModel>;
                    let models = unsafe { &*p_models };
                    let p_groups = p_groups as *const Vec<StaticInstanceGroup>;
                    let groups = unsafe { &*p_groups };

                    for range in &groups_sorted_by_technique[range.clone()] {
                        let model = &models[range.model_index];
                        for group_index in &range.instance_groups {
                            let group = &groups[*group_index];
                            if !group.visible {
                                continue;
                            }

                            group.cbuffer.bind_cbuffer(cmd, ShaderStage::Vertex, 1);
                            model.render_group(
                                cmd,
                                stage,
                                group.transforms.len() as u32,
                                range.group_index,
                            );
                        }
                    }
                });

            jobs.push(job);
        };

        let mut last_technique: Option<TagHash> = None;
        let mut last_range_start = 0;
        for (i, model_range) in groups_sorted_by_technique.iter().enumerate() {
            if last_technique.is_none() {
                last_technique = Some(groups_sorted_by_technique[i].technique);
            }

            if Some(model_range.technique) != last_technique {
                let range = last_range_start..i;
                schedule_range(range.clone());
                last_technique = Some(model_range.technique);
                last_range_start = i;
            }
        }

        if last_range_start < node_count {
            let range = last_range_start..node_count;
            schedule_range(range.clone());
        }

        // let Some(groups_sorted_by_technique) = self.groups_by_stage_sorted_by_technique.get(&stage)
        // else {
        //     for (model, _visible) in self.models.iter().filter(|(m, v)| {
        //         *v && m
        //             .model
        //             .special_meshes
        //             .iter()
        //             .any(|s| s.render_stage == stage)
        //     }) {
        //         let p_model = &raw const *model as u64;
        //         let pool_clone = renderer.cmd_pool.clone();
        //         let job = SCHEDULER
        //             .job_builder("static_geometry_special_meshes")
        //             .priority(Priority::High)
        //             .spawn(move || {
        //                 let model_ref = unsafe { &*(p_model as *const StaticModelRenderer) };
        //                 let cmd = pool_clone.get_command_list(set);
        //                 model_ref.render_all(cmd, stage);
        //             });

        //         jobs.push(job);
        //     }

        //     return;
        // };

        // let node_count = groups_sorted_by_technique.len();
        // // let nodes_per_job = node_count / job_count;
        // // let mut last_end = 0;
        // // let mut jobs_scheduled = 0;
        // let mut schedule_range = |range: std::ops::Range<usize>| {
        //     let groups_sorted_by_technique = groups_sorted_by_technique.clone();
        //     let p_models = &self.models as *const _ as u64;
        //     let pool_clone = renderer.cmd_pool.clone();
        //     let job = SCHEDULER
        //         .job_builder("static_geometry")
        //         .priority(Priority::High)
        //         .spawn(move || {
        //             let cmd = pool_clone.get_command_list(set);
        //             // Safety: p_models is valid for the lifetime of this closure
        //             // TODO(cohae): need a better way to pass self.models to the job
        //             let p_models = p_models as *const Vec<(StaticModelRenderer, bool)>;
        //             let models = unsafe { &*p_models };
        //             for (_technique_hash, model_index, group_index) in
        //                 &groups_sorted_by_technique[range.clone()]
        //             {
        //                 let (model, visible) = &models[*model_index];
        //                 if *visible {
        //                     model.render_group(cmd, stage, *group_index);
        //                 }
        //             }
        //         });

        //     jobs.push(job);
        // };

        // let mut last_technique: Option<TagHash> = None;
        // let mut last_range_start = 0;
        // for (i, (technique, _model_index, _group_index)) in
        //     groups_sorted_by_technique.iter().enumerate()
        // {
        //     if last_technique.is_none() {
        //         last_technique = Some(groups_sorted_by_technique[i].0);
        //     }

        //     if Some(*technique) != last_technique {
        //         let range = last_range_start..i;
        //         schedule_range(range.clone());
        //         last_technique = Some(*technique);
        //         last_range_start = i;
        //     }
        // }

        // if last_range_start < node_count {
        //     let range = last_range_start..node_count;
        //     schedule_range(range.clone());
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

struct SortedModel {
    technique: TagHash,
    model_index: usize,
    group_index: usize,
    instance_groups: SmallVec<[usize; 4]>,
}
