use alkahest_data::{
    statics::{SStaticMesh, SStaticSpecialMesh},
    tfx::{TfxRenderStage, TfxShaderStage},
};
use alkahest_pm::package_manager;
use destiny_pkg::TagHash;
use glam::Mat4;
use hecs::Entity;
use tiger_parse::PackageManagerExt;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT;

use crate::{
    ecs::{transform::Transform, Scene},
    gpu::{buffer::ConstantBuffer, GpuContext},
    handle::Handle,
    loaders::{index_buffer::IndexBuffer, vertex_buffer::VertexBuffer, AssetManager},
    tfx::{externs::ExternStorage, scope::ScopeInstances, technique::Technique},
};

pub(super) struct ModelBuffers {
    pub vertex0_buffer: Handle<VertexBuffer>,
    pub vertex1_buffer: Handle<VertexBuffer>,
    pub color_buffer: Handle<VertexBuffer>,
    pub index_buffer: Handle<IndexBuffer>,
}

impl ModelBuffers {
    pub fn bind(&self, asset_manager: &AssetManager, gctx: &GpuContext) -> Option<()> {
        unsafe {
            let vertex0 = asset_manager.vertex_buffers.get(&self.vertex0_buffer)?;
            let vertex1 = asset_manager.vertex_buffers.get(&self.vertex1_buffer)?;
            let color = asset_manager.vertex_buffers.get(&self.color_buffer);
            let index = asset_manager.index_buffers.get(&self.index_buffer)?;

            let ctx = gctx.context();
            ctx.IASetIndexBuffer(&index.buffer, DXGI_FORMAT(index.format as _), 0);
            ctx.IASetVertexBuffers(
                0,
                2,
                Some([Some(vertex0.buffer.clone()), Some(vertex1.buffer.clone())].as_ptr()),
                Some([vertex0.stride as _, vertex1.stride as _].as_ptr()),
                Some([0, 0].as_ptr()),
            );

            if let Some(color) = color {
                ctx.VSSetShaderResources(0, Some(&[color.srv.clone()]));
            }
        }

        Some(())
    }
}

struct SpecialMesh {
    mesh: SStaticSpecialMesh,
    buffers: ModelBuffers,
    technique: Handle<Technique>,
}

pub struct StaticModel {
    pub model: SStaticMesh,
    pub materials: Vec<Handle<Technique>>,
    buffers: Vec<ModelBuffers>,
    special_meshes: Vec<SpecialMesh>,
}

impl StaticModel {
    pub fn load(am: &mut AssetManager, tag: TagHash) -> anyhow::Result<Self> {
        let model = package_manager().read_tag_struct::<SStaticMesh>(tag)?;
        let materials = model
            .techniques
            .iter()
            .map(|&tag| am.get_or_load_technique(tag))
            .collect();

        let buffers = model
            .opaque_meshes
            .buffers
            .iter()
            .map(
                |&(index_buffer, vertex0_buffer, vertex1_buffer, color_buffer)| ModelBuffers {
                    vertex0_buffer: am.get_or_load_vertex_buffer(vertex0_buffer),
                    vertex1_buffer: am.get_or_load_vertex_buffer(vertex1_buffer),
                    color_buffer: am.get_or_load_vertex_buffer(color_buffer),
                    index_buffer: am.get_or_load_index_buffer(index_buffer),
                },
            )
            .collect();

        let special_meshes = model
            .special_meshes
            .iter()
            .map(|mesh| SpecialMesh {
                mesh: mesh.clone(),
                buffers: ModelBuffers {
                    vertex0_buffer: am.get_or_load_vertex_buffer(mesh.vertex0_buffer),
                    vertex1_buffer: am.get_or_load_vertex_buffer(mesh.vertex1_buffer),
                    color_buffer: am.get_or_load_vertex_buffer(mesh.color_buffer),
                    index_buffer: am.get_or_load_index_buffer(mesh.index_buffer),
                },
                technique: am.get_or_load_technique(mesh.technique),
            })
            .collect();

        Ok(Self {
            model,
            materials,
            buffers,
            special_meshes,
        })
    }

    /// ⚠ Expects the `instances` scope to be bound
    pub fn draw(
        &self,
        asset_manager: &AssetManager,
        externs: &ExternStorage,
        gctx: &GpuContext,
        render_stage: TfxRenderStage,
        instances_count: u32,
    ) {
        profiling::scope!("StaticModel::draw");
        for (i, group) in self
            .model
            .opaque_meshes
            .mesh_groups
            .iter()
            .enumerate()
            .filter(|(_, g)| g.render_stage == render_stage)
        {
            profiling::scope!("StaticModel::draw::group", format!("group_{}", i));
            let part = &self.model.opaque_meshes.parts[group.part_index as usize];
            if !part.lod_category.is_highest_detail() {
                continue;
            }

            let buffers = &self.buffers[part.buffer_index as usize];
            if buffers.bind(asset_manager, gctx).is_none() {
                continue;
            }

            let technique = &self.materials[i];
            if let Some(technique) = asset_manager.techniques.get(technique) {
                technique
                    .bind(gctx, externs, asset_manager)
                    .expect("Failed to bind technique");
            } else {
                continue;
            }

            gctx.set_input_layout(group.input_layout_index as usize);
            gctx.set_input_topology(part.primitive_type);

            unsafe {
                gctx.context().DrawIndexedInstanced(
                    part.index_count,
                    instances_count,
                    part.index_start,
                    0,
                    0,
                );
            }
        }

        self.draw_special_meshes(asset_manager, externs, gctx, render_stage);
    }

    fn draw_special_meshes(
        &self,
        asset_manager: &AssetManager,
        externs: &ExternStorage,
        gctx: &GpuContext,
        render_stage: TfxRenderStage,
    ) {
        profiling::scope!("StaticModel::draw_special_meshes");
        for mesh in self
            .special_meshes
            .iter()
            .filter(|m| m.mesh.render_stage == render_stage && m.mesh.lod.is_highest_detail())
        {
            if mesh.buffers.bind(asset_manager, gctx).is_none() {
                continue;
            }

            if let Some(technique) = asset_manager.techniques.get(&mesh.technique) {
                technique
                    .bind(gctx, externs, asset_manager)
                    .expect("Failed to bind technique");
                // } else {
                //     continue;
            }

            gctx.set_input_layout(mesh.mesh.input_layout_index as usize);
            gctx.set_input_topology(mesh.mesh.primitive_type);

            unsafe {
                gctx.context()
                    .DrawIndexed(mesh.mesh.index_count, mesh.mesh.index_start, 0);
            }
        }
    }
}

/// Parent of all static instances for a model
pub struct StaticInstances {
    pub instances: Vec<Entity>,
    pub model: StaticModel,
    pub cbuffer: ConstantBuffer<u8>,
}

impl StaticInstances {
    pub fn update_cbuffer(&self, scene: &Scene) {
        profiling::scope!("StaticInstances::update_cbuffer");
        let mut transforms = Vec::with_capacity(self.instances.len());
        for &instance in &self.instances {
            if let Ok(transform) = scene.get::<&Transform>(instance) {
                let mat = transform.to_mat4().transpose();
                transforms.push(Mat4::from_cols(
                    mat.x_axis.truncate().extend(transform.translation.x),
                    mat.y_axis.truncate().extend(transform.translation.y),
                    mat.z_axis.truncate().extend(transform.translation.z),
                    mat.w_axis,
                ));
            }
        }

        unsafe {
            let mesh_data = &self.model.model.opaque_meshes;
            self.cbuffer
                .write_array(
                    ScopeInstances {
                        mesh_offset: mesh_data.mesh_offset,
                        mesh_scale: mesh_data.mesh_scale,
                        uv_scale: mesh_data.texture_coordinate_scale,
                        uv_offset: mesh_data.texture_coordinate_offset,
                        max_color_index: mesh_data.max_color_index,
                        transforms,
                    }
                    .write()
                    .as_slice(),
                )
                .unwrap()
        }
    }
}

/// A single instance of a static model, can be manipulated individually
/// Rendered by [`StaticInstances`]
pub struct StaticInstance {
    pub parent: Entity,
}

pub fn draw_static_instances_system(
    gctx: &GpuContext,
    scene: &Scene,
    asset_manager: &AssetManager,
    externs: &ExternStorage,
    render_stage: TfxRenderStage,
) {
    profiling::scope!(
        "draw_static_instances_system",
        &format!("render_stage={render_stage:?}")
    );
    for (_, instances) in scene.query::<&StaticInstances>().iter() {
        // TODO(cohae): We want to pull the slot number from the `instances` scope
        instances.cbuffer.bind(1, TfxShaderStage::Vertex);
        instances.model.draw(
            asset_manager,
            externs,
            gctx,
            render_stage,
            instances.instances.len() as u32,
        );
    }
}

pub fn update_static_instances_system(scene: &Scene) {
    profiling::scope!("update_static_instances_system");
    for (_, instances) in scene.query::<&StaticInstances>().iter() {
        instances.update_cbuffer(scene);
    }
}
