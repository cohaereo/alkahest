use std::{any::Any, sync::Arc};

use alkahest_core::job::{SCHEDULER, potassium::JobHandle};
use alkahest_data::tfx::{
    RenderStage, ShaderStage, TfxScopeBits,
    common::AxisAlignedBBox,
    features::dynamic::{
        RenderStageSubscription, SDynamicMesh, SDynamicMeshMaterialVariants, SDynamicMeshPart,
        SDynamicModel,
    },
};
use anyhow::Context;
use glam::{Mat4, Vec4, Vec4Swizzles};
use itertools::{Itertools, multizip};
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

use super::{FeatureRenderer, shared::ModelBuffers};
use crate::{
    Renderer,
    asset::{Handle, handle::is_technique_loaded},
    gpu::{cbuffer::ConstantBuffer, command_list::CommandList},
    tfx::{
        expression_vm::interpreter::TempObjectChannels, packet::CompactTransform,
        technique::Technique,
    },
    util::threading::CommandListSetId,
};

pub struct DynamicModel {
    mesh_buffers: Vec<ModelBuffers>,

    technique_map: Vec<SDynamicMeshMaterialVariants>,
    techniques: Vec<Handle<Technique>>,

    pub model: SDynamicModel,
    pub mesh_stages: Vec<RenderStageSubscription>,
    pub subscribed_stages: RenderStageSubscription,
    part_techniques: Vec<Vec<Handle<Technique>>>,

    // pub selected_mesh: usize,
    pub permutation: usize,
    permutation_count: usize,

    identifier_count: usize,

    pub hash: TagHash,

    pub cb: ConstantBuffer<RigidModelConstants>,
    pub channels: TempObjectChannels,
    pub transform: Mat4,
}

impl DynamicModel {
    #[profiling::function]
    pub fn load(
        hash: TagHash,
        technique_map: Vec<SDynamicMeshMaterialVariants>,
        techniques: Vec<TagHash>,
    ) -> anyhow::Result<Box<Self>> {
        let model = package_manager().read_tag_struct::<SDynamicModel>(hash)?;

        let techniques = techniques
            .iter()
            .map(|&tag| Renderer::instance().asset_manager.load(tag))
            .collect_vec();

        let mesh_buffers = model
            .meshes
            .iter()
            .map(|m| {
                ModelBuffers::load(
                    m.vertex0_buffer,
                    m.vertex1_buffer,
                    m.color_buffer,
                    m.index_buffer,
                )
                .expect("Failed to load model buffers for dynamic model")
            })
            .collect_vec();

        let mesh_stages = model
            .meshes
            .iter()
            .map(|m| RenderStageSubscription::from_partrange_list(&m.part_range_per_render_stage))
            .collect_vec();

        let part_techniques = model
            .meshes
            .iter()
            .map(|m| {
                m.parts
                    .iter()
                    .map(|p| Renderer::instance().asset_manager.load(p.technique))
                    .collect_vec()
            })
            .collect_vec();

        let permutation_count = technique_map
            .iter()
            .filter(|m| m.unk8 == 0)
            .map(|m| m.technique_count as usize)
            .next()
            .unwrap_or(1);

        let identifier_count = model
            .meshes
            .iter()
            .map(|m| {
                m.parts
                    .iter()
                    .map(|p| p.external_identifier)
                    .max()
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0) as usize
            + 1;

        Ok(Box::new(Self {
            permutation: permutation_count - 1,
            permutation_count,
            // selected_mesh: 0,
            identifier_count,
            mesh_buffers,
            technique_map,
            techniques,
            model,
            subscribed_stages: mesh_stages
                .iter()
                .fold(RenderStageSubscription::empty(), |acc, &x| acc | x),
            mesh_stages,
            part_techniques,
            hash,
            cb: ConstantBuffer::create(&Renderer::instance().gpu, None)
                .context("Failed to create constant buffer")?,
            channels: TempObjectChannels::default(),
            transform: Mat4::IDENTITY,
        }))
    }

    pub fn mesh_count(&self) -> usize {
        self.model.meshes.len()
    }

    pub fn variant_count(&self) -> usize {
        self.permutation_count
    }

    pub fn identifier_count(&self) -> usize {
        self.identifier_count
    }

    fn get_permutation_technique(
        &self,
        index: u16,
        permutation_count: usize,
    ) -> Option<Handle<Technique>> {
        if index == u16::MAX {
            None
        } else {
            self.technique_map
                .get(index as usize)
                .as_ref()
                .map(|permutation_range| {
                    self.techniques[permutation_range.technique_start as usize
                        + (permutation_count % permutation_range.technique_count as usize)]
                        .clone()
                })
        }
    }

    // /// ⚠ Expects the `rigid_model` scope to be bound
    // pub fn draw(
    //     &self,
    //     renderer: &Renderer,
    //     render_stage: TfxRenderStage,
    //     identifier: u16,
    //     object_channels: Option<&ObjectChannels>,
    // ) -> anyhow::Result<()> {
    //     self.draw_wrapped(
    //         renderer,
    //         render_stage,
    //         identifier,
    //         object_channels,
    //         |_, renderer, _mesh, part| unsafe {
    //             renderer
    //                 .gpu
    //                 .lock_context()
    //                 .DrawIndexed(part.index_count, part.index_start, 0);
    //         },
    //     )
    // }

    pub fn draw_wrapped<F>(
        &self,
        cmd: &mut CommandList,
        stage: RenderStage,
        identifier: u16,
        mut f: F,
    ) where
        F: FnMut(&Self, &mut CommandList, (usize, &SDynamicMesh), (usize, &SDynamicMeshPart)),
    {
        for (mesh_index, (mesh, subscribed_stages, mesh_buffers, mesh_techniques)) in multizip((
            self.model.meshes.iter(),
            self.mesh_stages.iter(),
            self.mesh_buffers.iter(),
            self.part_techniques.iter(),
        ))
        .enumerate()
        {
            if !subscribed_stages.is_subscribed(stage) {
                continue;
            }

            self.cb.bind(cmd, ShaderStage::Vertex, 1);
            self.cb.bind(cmd, ShaderStage::Pixel, 1);

            cmd.set_input_layout(mesh.get_input_layout_for_stage(stage) as usize);
            mesh_buffers.bind(cmd);
            for part_index in mesh.get_range_for_stage(stage) {
                let part = &mesh.parts[part_index];
                if identifier != u16::MAX && part.external_identifier != identifier {
                    continue;
                }

                if !part.lod_category.is_highest_detail() {
                    continue;
                }

                let variant_material =
                    self.get_permutation_technique(part.variant_shader_index, self.permutation);

                let mut all_scopes = TfxScopeBits::empty();
                if let Some(technique) = mesh_techniques[part_index].get() {
                    technique
                        .bind_with_channels(cmd, Some(&self.channels))
                        .expect("Failed to bind technique");
                    all_scopes |= technique.used_scopes;
                }

                if let Some(technique) = &variant_material {
                    if let Some(tech) = technique.get() {
                        tech.bind_with_channels(cmd, Some(&self.channels))
                            .expect("Failed to bind variant technique");
                        all_scopes |= tech.used_scopes;
                    }
                }

                // No technique, no scopes, no draw
                if all_scopes.is_empty() {
                    continue;
                }

                if all_scopes.contains(TfxScopeBits::SKINNING) {
                    cmd.vertex_set_shader(&Renderer::instance().common.disable_skinning_vs);
                }

                cmd.set_input_topology(part.primitive_type);

                f(self, cmd, (mesh_index, mesh), (part_index, part));
            }
        }
    }
}

#[profiling::all_functions]
impl FeatureRenderer for DynamicModel {
    fn visibility_test(&mut self, camera: &crate::camera::Camera) -> bool {
        // TODO(cohae): frustum culling is broken for some moving models (such as the vertex animated fan segments in Irkalla Complex)
        let bounds = AxisAlignedBBox::from_center_extents(
            self.model.model_offset.xyz(),
            self.model.model_scale.xyz() * 2.0,
        )
        .transformed(self.transform);

        camera.is_visible(&bounds)
    }

    fn extract_and_prepare(&mut self, renderer: &Renderer, extracted_data: &dyn Any) {
        let (obj_local_to_world, permutation) = extracted_data
            .downcast_ref::<(CompactTransform, usize)>()
            .expect("Invalid extracted data type")
            .clone();
        self.transform = obj_local_to_world.to_mat4();
        self.permutation = permutation;

        self.cb
            .write(
                &renderer.gpu.context(),
                &RigidModelConstants {
                    mesh_to_world: obj_local_to_world.to_mat4(),
                    position_scale: self.model.model_scale,
                    position_offset: self.model.model_offset,
                    texcoord0_scale_offset: Vec4::new(
                        self.model.texcoord_scale.x,
                        self.model.texcoord_scale.y,
                        self.model.texcoord_offset.x,
                        self.model.texcoord_offset.y,
                    ),
                    dynamic_sh_ao_values: Vec4::new(0.0, 0.0, 0.0, 0.8),
                },
            )
            .unwrap();

        self.channels.position = obj_local_to_world.translation().extend(0.0);
    }

    // #[profiling::function]
    fn submit(&self, cmd: &mut CommandList, stage: RenderStage) {
        profiling::scope!("DynamicModel::draw");

        self.draw_wrapped(
            cmd,
            stage,
            u16::MAX,
            |_model, cmd, (_mesh_index, _mesh), (_part_index, part)| {
                cmd.draw_indexed(part.index_count, part.index_start, 0);
            },
        );
    }

    fn submit_parallel(
        &self,
        renderer: &Arc<Renderer>,
        set: CommandListSetId,
        stage: RenderStage,
        jobs: &mut Vec<JobHandle>,
    ) {
        let self_p = &raw const *self as u64;
        let pool = renderer.cmd_pool.clone();
        let job = SCHEDULER.job_builder("rigid_model").spawn(move || {
            let self_ref = unsafe { &*(self_p as *const Self) };
            let cmd = pool.get_command_list(set);
            self_ref.draw_wrapped(
                cmd,
                stage,
                u16::MAX,
                |_model, cmd, (_mesh_index, _mesh), (_part_index, part)| {
                    cmd.draw_indexed(part.index_count, part.index_start, 0);
                },
            );
        });
        jobs.push(job);
    }

    fn subscribed_stages(&self) -> RenderStageSubscription {
        self.subscribed_stages
    }

    fn is_loaded(&self) -> bool {
        if self
            .part_techniques
            .iter()
            .any(|v| v.iter().any(|t| !is_technique_loaded(t)))
        {
            return false;
        }

        if self.techniques.iter().any(|t| !is_technique_loaded(t)) {
            return false;
        }

        true
    }
}

#[repr(C)]
pub struct RigidModelConstants {
    pub mesh_to_world: Mat4,          // c0-c3
    pub position_scale: Vec4,         // c4
    pub position_offset: Vec4,        // c5
    pub texcoord0_scale_offset: Vec4, // c6
    pub dynamic_sh_ao_values: Vec4,   // c7
}
