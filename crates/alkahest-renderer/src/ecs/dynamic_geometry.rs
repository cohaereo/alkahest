use alkahest_data::{
    entity::{SDynamicMeshPart, SDynamicModel, Unk808072c5},
    statics::{SStaticMesh, SStaticSpecialMesh},
    tfx::{TfxRenderStage, TfxShaderStage},
};
use alkahest_pm::package_manager;
use anyhow::ensure;
use destiny_pkg::TagHash;
use glam::{Mat4, Vec3, Vec4};
use hecs::Entity;
use tiger_parse::PackageManagerExt;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT;

use crate::{
    ecs::{
        static_geometry::{ModelBuffers, StaticInstances},
        transform::Transform,
        Scene,
    },
    gpu::{buffer::ConstantBuffer, GpuContext},
    handle::Handle,
    loaders::{index_buffer::IndexBuffer, vertex_buffer::VertexBuffer, AssetManager},
    tfx::{
        externs::ExternStorage,
        scope::{ScopeInstances, ScopeRigidModel},
        technique::Technique,
        view::RenderStageSubscriptions,
    },
};

pub struct DynamicModel {
    mesh_buffers: Vec<ModelBuffers>,

    technique_map: Vec<Unk808072c5>,
    techniques: Vec<Handle<Technique>>,

    pub model: SDynamicModel,
    pub mesh_stages: Vec<RenderStageSubscriptions>,
    part_techniques: Vec<Vec<Handle<Technique>>>,
}

impl DynamicModel {
    pub fn load(
        am: &mut AssetManager,
        tag: TagHash,
        technique_map: Vec<Unk808072c5>,
        techniques: Vec<TagHash>,
    ) -> anyhow::Result<Self> {
        let model = package_manager().read_tag_struct::<SDynamicModel>(tag)?;
        let techniques = techniques
            .iter()
            .map(|&tag| am.get_or_load_technique(tag))
            .collect();

        let mesh_buffers = model
            .meshes
            .iter()
            .map(|m| ModelBuffers {
                vertex0_buffer: am.get_or_load_vertex_buffer(m.vertex0_buffer),
                vertex1_buffer: am.get_or_load_vertex_buffer(m.vertex1_buffer),
                color_buffer: am.get_or_load_vertex_buffer(m.color_buffer),
                index_buffer: am.get_or_load_index_buffer(m.index_buffer),
            })
            .collect();

        let mesh_stages = model
            .meshes
            .iter()
            .map(|m| RenderStageSubscriptions::from_partrange_list(&m.part_range_per_render_stage))
            .collect();

        let part_techniques = model
            .meshes
            .iter()
            .map(|m| {
                m.parts
                    .iter()
                    .map(|p| am.get_or_load_technique(p.technique))
                    .collect()
            })
            .collect();

        Ok(Self {
            mesh_buffers,
            technique_map,
            techniques,
            model,
            mesh_stages,
            part_techniques,
        })
    }

    pub fn mesh_count(&self) -> usize {
        self.model.meshes.len()
    }

    fn get_variant_technique(&self, index: u16, variant: usize) -> Option<Handle<Technique>> {
        if index == u16::MAX {
            None
        } else {
            let variant_range = &self.technique_map[index as usize];
            Some(
                self.techniques[variant_range.technique_start as usize
                    + (variant % variant_range.technique_count as usize)]
                    .clone(),
            )
        }
    }

    /// ⚠ Expects the `rigid_model` scope to be bound
    pub fn draw(
        &self,
        asset_manager: &AssetManager,
        externs: &ExternStorage,
        gctx: &GpuContext,
        render_stage: TfxRenderStage,
        mesh_index: usize,
    ) -> anyhow::Result<()> {
        profiling::scope!("DynamicModel::draw", format!("mesh={mesh_index}"));
        ensure!(mesh_index < self.mesh_count(), "Invalid mesh index");

        let mesh = &self.model.meshes[mesh_index];
        let stages = &self.mesh_stages[mesh_index];
        if !stages.is_subscribed(render_stage) {
            return Ok(());
        }

        gctx.set_input_layout(mesh.get_input_layout_for_stage(render_stage) as usize);
        self.mesh_buffers[mesh_index].bind(asset_manager, gctx);
        for part_index in mesh.get_range_for_stage(render_stage) {
            let part = &mesh.parts[part_index];
            if !part.lod_category.is_highest_detail() {
                continue;
            }

            let variant_material = self.get_variant_technique(part.variant_shader_index, 0);

            if let Some(technique) = asset_manager
                .techniques
                .get(&self.part_techniques[mesh_index][part_index])
            {
                technique
                    .bind(gctx, externs, asset_manager)
                    .expect("Failed to bind technique");
                // } else {
                //     continue;
            }

            if let Some(technique) = variant_material.and_then(|t| asset_manager.techniques.get(&t))
            {
                technique
                    .bind(gctx, externs, asset_manager)
                    .expect("Failed to bind variant technique");
            }

            if stages.contains(RenderStageSubscriptions::COMPUTE_SKINNING) {
                unsafe {
                    gctx.context()
                        .VSSetShader(&gctx.util_resources.entity_vs_override, None);
                }
            }

            gctx.set_input_topology(part.primitive_type);

            unsafe {
                gctx.context()
                    .DrawIndexed(part.index_count, part.index_start, 0);
            }
        }

        Ok(())
    }
}

pub struct DynamicModelComponent {
    pub model: DynamicModel,
    pub cbuffer: ConstantBuffer<ScopeRigidModel>,
}

pub fn draw_dynamic_model_system(
    gctx: &GpuContext,
    scene: &Scene,
    asset_manager: &AssetManager,
    externs: &ExternStorage,
    render_stage: TfxRenderStage,
) {
    profiling::scope!(
        "draw_dynamic_model_system",
        &format!("render_stage={render_stage:?}")
    );
    for (_, dynamic) in scene.query::<&DynamicModelComponent>().iter() {
        // TODO(cohae): We want to pull the slot number from the `rigid_model` scope
        dynamic.cbuffer.bind(1, TfxShaderStage::Vertex);
        // TODO(cohae): Error reporting
        dynamic
            .model
            .draw(asset_manager, externs, gctx, render_stage, 0)
            .unwrap();
    }
}

pub fn update_dynamic_model_system(scene: &Scene) {
    profiling::scope!("update_dynamic_model_system");
    for (_, (transform, model)) in scene.query::<(&Transform, &DynamicModelComponent)>().iter() {
        let mm = transform.to_mat4();

        let model_matrix = Mat4::from_cols(
            mm.x_axis.truncate().extend(mm.w_axis.x),
            mm.y_axis.truncate().extend(mm.w_axis.y),
            mm.z_axis.truncate().extend(mm.w_axis.z),
            mm.w_axis,
        );

        let alt_matrix = Mat4::from_cols(
            Vec3::ONE.extend(mm.w_axis.x),
            Vec3::ONE.extend(mm.w_axis.y),
            Vec3::ONE.extend(mm.w_axis.z),
            Vec4::W,
        );

        model
            .cbuffer
            .write(&ScopeRigidModel {
                mesh_to_world: model_matrix,
                position_scale: model.model.model.model_scale,
                position_offset: model.model.model.model_offset,
                texcoord0_scale_offset: Vec4::new(
                    model.model.model.texcoord_scale.x,
                    model.model.model.texcoord_scale.y,
                    model.model.model.texcoord_offset.x,
                    model.model.model.texcoord_offset.y,
                ),
                dynamic_sh_ao_values: Vec4::new(1.0, 1.0, 1.0, 0.0),
                unk8: [alt_matrix; 8],
            })
            .unwrap();
    }
}
