use std::{any::Any, f32, io::Write, ops::Deref, sync::Arc};

use ahash::HashMap;
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
use glam::{Mat4, Quat, Vec3, Vec4};
use itertools::Itertools;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use smallvec::SmallVec;
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

use super::{FeatureRenderer, shared::ModelBuffers};
use crate::{
    Gpu, Renderer,
    asset::{Handle, handle::is_technique_loaded},
    gpu::{cbuffer::ConstantBuffer, command_list::CommandList},
    tfx::{
        packet::{CompactTransform, VisibilityMask},
        technique::Technique,
        view::View,
    },
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

pub struct StaticMesh {
    pub model: SStaticMesh,
    pub materials: Vec<Handle<Technique>>,
    pub hash: TagHash,
    pub subscribed_stages: RenderStageSubscription,
    buffers: Vec<ModelBuffers>,
    special_meshes: Vec<SpecialMesh>,
}

impl StaticMesh {
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
                    technique.bind(cmd).expect("Failed to bind technique");
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
                    technique.bind(cmd).expect("Failed to bind technique");
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

    // #[profiling::function]
    /// The draw closure is called with (cmd, part.index_count, part.index_start)
    pub fn render_group<F>(
        &self,
        cmd: &mut CommandList,
        stage: RenderStage,
        group: usize,
        bind_technique: bool,
        draw: F,
    ) where
        F: Fn(&mut CommandList, u32, u32),
    {
        profiling::scope!(
            "render static model group",
            &format!("model={}, group={}", self.hash, group)
        );

        let i = group;
        let group = &self.model.opaque_meshes.mesh_groups[i];
        if group.render_stage != stage {
            return;
        }

        let part = &self.model.opaque_meshes.parts[group.part_index as usize];
        if !part.lod_category.is_highest_detail() {
            return;
        }

        {
            profiling::scope!(
                "bind buffers",
                &format!("buffer_index={}", part.buffer_index)
            );
            let buffers = &self.buffers[part.buffer_index as usize];
            if buffers.bind(cmd).is_none() {
                return;
            }
        }

        if let Some(technique) = &self.materials.get(i).and_then(Handle::get) {
            if bind_technique {
                technique.bind(cmd).expect("Failed to bind technique");
            }
        } else {
            return;
        }

        cmd.set_input_layout(group.input_layout_index as usize);
        cmd.set_input_topology(part.primitive_type);

        // cmd.draw_indexed_instanced(part.index_count, instance_count, part.index_start, 0, 0);
        draw(cmd, part.index_count, part.index_start);
    }
}

struct StaticInstanceGroup {
    pub transforms: Vec<(SStaticInstanceTransform, VisibilityMask)>,
    pub static_index: u16,
    pub cbuffer: ConstantBuffer<u8>,
    pub bounds: Vec<AxisAlignedBBox>,
    pub group_bounds: AxisAlignedBBox,
    pub visible: VisibilityMask,
    pub num_instances: u32,
    num_instances_written: u32,
}

impl StaticInstanceGroup {
    #[profiling::function]
    pub fn update_constants(
        &self,
        ctx: &d3d11::DeviceContext,
        view_index: usize,
        model: &StaticMesh,
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
        for transform in self
            .transforms
            .iter()
            .filter_map(|(t, visible)| visible.get(view_index).then_some(t))
        {
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

    static_models: Vec<StaticMesh>,
    groups: Vec<StaticInstanceGroup>,

    vao_identifier: u64,
    groups_by_stage_sorted_by_technique: HashMap<RenderStage, Arc<Vec<SortedModel>>>,
    bounds: AxisAlignedBBox,
}

impl StaticInstancesRenderer {
    pub fn load(gpu: &Arc<Gpu>, instances_hash: TagHash) -> anyhow::Result<Self> {
        let instances: SStaticMeshInstances = package_manager().read_tag_struct(instances_hash)?;
        let mut static_models = Vec::with_capacity(instances.statics.len());

        for model in &instances.statics {
            let renderer = StaticMesh::load(*model)?;

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
                .iter()
                .cloned()
                .map(|t| (t, VisibilityMask::default()))
                .collect_vec();

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
                bounds.push(b.bb);
            }

            let group_bounds = bounds.iter().cloned().sum();

            let cbuffer = ConstantBuffer::create_raw(
                gpu,
                2 * size_of::<Vec4>() // quantization headers
                            + transforms.len() * size_of::<InstanceTransformBlock>(), // per-transform data
            )?;

            groups.push(StaticInstanceGroup {
                num_instances: transforms.len() as u32,
                transforms,
                bounds,
                group_bounds,
                static_index: group.static_index,
                cbuffer,
                visible: VisibilityMask::default(),
                num_instances_written: 0,
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
            vao_identifier: instances.vertex_ao_identifier,
            groups_by_stage_sorted_by_technique,
            bounds: instances.bounds,
        })
    }
}

#[profiling::all_functions]
impl FeatureRenderer for StaticInstancesRenderer {
    fn visibility_test(&mut self, view_index: usize, view: &View) -> bool {
        if !view.is_visible(&self.bounds) {
            return false;
        }

        let enable_instance_culling = Renderer::instance().settings().instance_culling;
        self.groups.par_iter_mut().for_each(|group| {
            let StaticInstanceGroup {
                transforms, bounds, ..
            } = group;

            group
                .visible
                .set(view_index, view.is_visible(&group.group_bounds));
            group.num_instances = 0;
            if group.visible.get(view_index) {
                group.visible.set(view_index, false);
                for ((_transform, visible), bounds) in transforms.iter_mut().zip(bounds.iter()) {
                    let bounds_visible = view.is_visible(bounds);

                    if enable_instance_culling {
                        visible.set(view_index, bounds_visible);
                        if visible.get(view_index) {
                            group.num_instances += 1;
                            group.visible.set_or(view_index, true);
                        }
                    } else {
                        visible.set(view_index, true);
                        group.num_instances += 1;
                        group.visible.set_or(view_index, bounds_visible);
                    }
                }
            }
        });

        self.groups.iter().any(|m| m.visible.get(view_index))
    }

    fn prepare(
        &mut self,
        renderer: &Renderer,
        view_index: usize,
        _extracted_data: &dyn std::any::Any,
    ) {
        for group in self.groups.iter_mut().filter(|g| g.visible.get(view_index)) {
            // If all instances are visible and written already, don't bother updating
            if group.num_instances == group.num_instances_written
                && group.num_instances_written as usize == group.transforms.len()
            {
                continue;
            } else {
                let model = &self.static_models[group.static_index as usize];
                group.update_constants(
                    &renderer.gpu.context(),
                    view_index,
                    model,
                    self.vao_identifier,
                    renderer.ao.read().as_ref(),
                );
                group.num_instances_written = group.num_instances;
            }
        }
    }

    fn submit(&self, cmd: &mut CommandList, view_index: usize, stage: RenderStage) {
        for group in self.groups.iter().filter(|g| {
            let model = &self.static_models[g.static_index as usize];

            g.visible.get(view_index) && model.subscribed_stages.is_subscribed(stage)
        }) {
            let model = &self.static_models[group.static_index as usize];
            group.cbuffer.bind_cbuffer(cmd, ShaderStage::Vertex, 1);
            model.render_all(cmd, stage, group.num_instances);
        }
    }

    fn submit_parallel(
        &self,
        _renderer: &Arc<Renderer>,
        view_index: usize,
        set: CommandListSetId,
        stage: RenderStage,
        jobs: &mut Vec<JobHandle>,
    ) {
        let renderer = Renderer::instance();

        let Some(groups_sorted_by_technique) = self.groups_by_stage_sorted_by_technique.get(&stage)
        else {
            for (i, _group) in self.groups.iter().enumerate().filter(|(_i, g)| {
                let model = &self.static_models[g.static_index as usize];
                g.visible.get(view_index) && model.subscribed_stages.is_subscribed(stage)
            }) {
                let p_models = &self.static_models as *const _ as u64;
                let p_groups = &self.groups as *const _ as u64;
                let pool_clone = renderer.cmd_pool.clone();
                let job = SCHEDULER
                    .job_builder("static_geometry")
                    .priority(Priority::High)
                    .spawn(move || {
                        let cmd = pool_clone.get_command_list(set);
                        let renderer = Renderer::instance();
                        if let Some(ao_vb) =
                            renderer.ao_buffer.read().as_ref().and_then(|h| h.get())
                        {
                            cmd.vertex_set_shader_resources(
                                1,
                                std::slice::from_ref(&ao_vb.srv.as_ref()),
                            );
                        }

                        // Safety: p_models/p_groups are (practically) valid for the lifetime of this closure
                        // TODO(cohae): need a safer way to pass self.models to the job
                        let p_models = p_models as *const Vec<StaticMesh>;
                        let models = unsafe { &*p_models };
                        let p_groups = p_groups as *const Vec<StaticInstanceGroup>;
                        let groups = unsafe { &*p_groups };
                        let group = &groups[i];
                        let model = &models[group.static_index as usize];
                        group.cbuffer.bind_cbuffer(cmd, ShaderStage::Vertex, 1);
                        model.render_all(cmd, stage, group.num_instances);
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
                group_indices
                    .iter()
                    .any(|&gi| self.groups[gi].visible.get(view_index))
            });

            if !visible {
                return;
            }

            let job = SCHEDULER
                .job_builder("static_geometry")
                .priority(Priority::High)
                .spawn(move || {
                    let cmd = pool_clone.get_command_list(set);

                    let renderer = Renderer::instance();
                    if let Some(ao_vb) = renderer.ao_buffer.read().as_ref().and_then(|h| h.get()) {
                        cmd.vertex_set_shader_resources(
                            1,
                            std::slice::from_ref(&ao_vb.srv.as_ref()),
                        );
                    }

                    // Safety: p_models/p_groups are (practically) valid for the lifetime of this closure
                    // TODO(cohae): need a safer way to pass self.models to the job
                    let p_models = p_models as *const Vec<StaticMesh>;
                    let models = unsafe { &*p_models };
                    let p_groups = p_groups as *const Vec<StaticInstanceGroup>;
                    let groups = unsafe { &*p_groups };

                    let mut bind_technique = true;
                    for range in &groups_sorted_by_technique[range.clone()] {
                        let model = &models[range.model_index];

                        model.render_group(
                            cmd,
                            stage,
                            range.group_index,
                            bind_technique,
                            |cmd, index_count, index_start| {
                                for group_index in &range.instance_groups {
                                    let group = &groups[*group_index];
                                    group.cbuffer.bind_cbuffer(cmd, ShaderStage::Vertex, 1);
                                    if group.visible.get(view_index) {
                                        cmd.draw_indexed_instanced(
                                            index_count,
                                            group.num_instances,
                                            index_start,
                                            0,
                                            0,
                                        );
                                    }
                                }
                            },
                        );

                        bind_technique = false;
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

    fn is_loaded(&self) -> bool {
        if self
            .static_models
            .iter()
            .any(|v| v.materials.iter().any(|t| !is_technique_loaded(t)))
        {
            return false;
        }

        if self.static_models.iter().any(|v| {
            v.special_meshes
                .iter()
                .any(|s| !is_technique_loaded(&s.technique))
        }) {
            return false;
        }

        true
    }
}

struct SortedModel {
    technique: TagHash,
    model_index: usize,
    group_index: usize,
    instance_groups: SmallVec<[usize; 4]>,
}

pub struct StaticModelRenderer {
    model: StaticMesh,
    group: StaticInstanceGroup,
    pub bounds: AxisAlignedBBox,
}

impl StaticModelRenderer {
    pub fn new(gpu: &Gpu, model: StaticMesh) -> anyhow::Result<Self> {
        let cbuffer = ConstantBuffer::create_raw(
            gpu,
            2 * size_of::<Vec4>() // quantization headers
                        +  size_of::<InstanceTransformBlock>(), // per-transform data
        )?;

        let om = &model.model.opaque_meshes;
        let bounds =
            AxisAlignedBBox::from_center_extents(om.mesh_offset, Vec3::splat(om.mesh_scale));

        let group = StaticInstanceGroup {
            transforms: vec![(
                SStaticInstanceTransform {
                    rotation: Quat::IDENTITY,
                    translation: Vec3::ZERO,
                    scale: 1.0,
                    unk20: [0; 2],
                    vertex_ao_offset: 0,
                    unk2c: 0.0,
                    unk30: [0; 4],
                },
                VisibilityMask::default(),
            )],
            static_index: 0,
            cbuffer,
            bounds: vec![],
            group_bounds: AxisAlignedBBox::NONE,
            visible: VisibilityMask::default(),
            num_instances: 1,
            num_instances_written: 1,
        };

        Ok(Self {
            model,
            group,
            bounds,
        })
    }
}

impl FeatureRenderer for StaticModelRenderer {
    fn prepare(&mut self, renderer: &Renderer, view_index: usize, extracted_data: &dyn Any) {
        let (obj_local_to_world, _permutation) = extracted_data
            .downcast_ref::<(CompactTransform, usize)>()
            .expect("Invalid extracted data type")
            .clone();
        let transform = obj_local_to_world.to_mat4();

        let (scale, rotation, translation) = transform.to_scale_rotation_translation();
        let transform = &mut self.group.transforms[0].0;
        transform.rotation = rotation;
        transform.translation = translation;
        transform.scale = scale.x;

        self.group
            .update_constants(&renderer.gpu.context(), view_index, &self.model, 0, None);
    }

    fn submit(&self, cmd: &mut CommandList, _view_index: usize, stage: RenderStage) {
        self.group.cbuffer.bind_cbuffer(cmd, ShaderStage::Vertex, 1);
        self.model.render_all(cmd, stage, 1);
    }

    fn submit_parallel(
        &self,
        renderer: &Arc<Renderer>,
        _view_index: usize,
        set: CommandListSetId,
        stage: RenderStage,
        jobs: &mut Vec<JobHandle>,
    ) {
        let self_p = &raw const *self as u64;
        let pool = renderer.cmd_pool.clone();
        let job = SCHEDULER.job_builder("rigid_model").spawn(move || {
            let self_ref = unsafe { &*(self_p as *const Self) };
            let cmd = pool.get_command_list(set);
            self_ref
                .group
                .cbuffer
                .bind_cbuffer(cmd, ShaderStage::Vertex, 1);
            self_ref.model.render_all(cmd, stage, 1);
        });
        jobs.push(job);
    }

    fn subscribed_stages(&self) -> RenderStageSubscription {
        self.model.subscribed_stages
    }
}
