use std::{f32, io::Write, ops::Deref, sync::Arc};

use alkahest_data::tfx::{
    common::AxisAlignedBBox,
    features::{
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
use itertools::Itertools;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use tiger_parse::PackageManagerExt;
use tiger_pkg::{package_manager, TagHash};

use super::{shared::ModelBuffers, FeatureRenderer};
use crate::{
    asset::Handle,
    camera::Camera,
    gpu::{cbuffer::ConstantBuffer, command_list::CommandList, state::GpuState},
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
        // ao: Option<&SStaticAmbientOcclusion>,
    ) {
        let mut buffer = vec![];
        let model = &self.model.model.opaque_meshes;

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

            // let matrix = instance_transform;
            // let vertex_ao_offset = if let Some(vao_base) = vao_base {
            //     transform.vertex_ao_offset + vao_base
            // } else {
            //     transform.vertex_ao_offset
            // };

            buffer
                .write_all(bytemuck::cast_slice(&[InstanceTransformBlock {
                    transform: [
                        instance_transform.x_axis,
                        instance_transform.y_axis,
                        instance_transform.z_axis,
                    ],
                    params: Vec4::new(
                        1.0,
                        1.0,
                        1.0,
                        f32::from_bits(0x02000000),
                        // f32::from_bits(
                        //     ao_offsets
                        //         .get(i)
                        //         .copied()
                        //         .map(|v| v.shr(2))
                        //         .unwrap_or(0x02000000),
                        // ),
                    ),
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
                model.update_constants(
                    &renderer.gpu.context(), /*, renderer.ao.read().as_ref() */
                );
                model.constants_dirty = false;
            }
        }
    }

    fn submit(&self, cmd: &mut CommandList, stage: RenderStage) {
        let Some(groups_sorted_by_technique) = self.groups_by_stage_sorted_by_technique.get(&stage)
        else {
            // Special meshes are rendered single-threaded for now
            for (model, _visible) in self.models.iter().filter(|(_, v)| *v) {
                model.render_all(cmd, stage);
            }
            return;
        };

        let initial_state = Arc::new(GpuState::backup(cmd));

        // Equally divide groups_sorted_by_technique into X ranges for parallel processing
        // let mut job_ranges = vec![];
        let job_count = 6;
        let node_count = groups_sorted_by_technique.len();
        let nodes_per_job = node_count / job_count;
        let mut last_end = 0;
        let mut jobs_scheduled = 0;
        for _i in 0..job_count {
            let node_start = last_end;
            let mut node_end = (node_start + nodes_per_job).min(node_count);

            if node_start >= node_count || node_end == node_start {
                break;
            }

            let last_technique = groups_sorted_by_technique[node_end - 1].0;
            // Extend node_end to include all groups with the same technique hash
            loop {
                if node_end < node_count && groups_sorted_by_technique[node_end].0 == last_technique
                {
                    node_end += 1;
                } else {
                    break;
                }
            }

            last_end = node_end;
            let range = node_start..node_end;
            // job_ranges.push(node_start..node_end);

            let groups_sorted_by_technique = groups_sorted_by_technique.clone();
            let initial_state = initial_state.clone();
            let p_models = &self.models as *const _ as u64;
            Renderer::instance()
                .cmd_pool
                .queue_job(Box::new(move |job_cmd: &mut CommandList| {
                    profiling::scope!("command_thread_job", &format!("{range:?}"));
                    // Safety: p_models is valid for the lifetime of this closure
                    // TODO(cohae): need a better way to pass self.models to the job
                    let p_models = p_models as *const Vec<(StaticModelRenderer, bool)>;
                    let models = unsafe { &*p_models };
                    initial_state.restore(job_cmd);
                    for (_technique_hash, model_index, group_index) in
                        &groups_sorted_by_technique[range.clone()]
                    {
                        let (model, visible) = &models[*model_index];
                        if *visible {
                            model.render_group(job_cmd, stage, *group_index);
                        }
                    }
                }));
            jobs_scheduled += 1;
        }

        for cmd_result in Renderer::instance()
            .cmd_pool
            .collect_results(jobs_scheduled)
        {
            cmd.execute_command_list(&cmd_result, true);
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
