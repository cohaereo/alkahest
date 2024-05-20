use std::sync::Arc;

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
    ecs::{common::Hidden, hierarchy::Children, transform::Transform, Scene},
    gpu::{buffer::ConstantBuffer, GpuContext, SharedGpuContext},
    gpu_event,
    handle::Handle,
    loaders::{index_buffer::IndexBuffer, vertex_buffer::VertexBuffer, AssetManager},
    renderer::Renderer,
    tfx::{scope::ScopeInstances, technique::Technique, view::RenderStageSubscriptions},
    util::packages::TagHashExt,
};

pub(super) struct ModelBuffers {
    pub vertex0_buffer: Handle<VertexBuffer>,
    pub vertex1_buffer: Handle<VertexBuffer>,
    pub color_buffer: Handle<VertexBuffer>,
    pub index_buffer: Handle<IndexBuffer>,
}

impl ModelBuffers {
    pub fn bind(&self, renderer: &Renderer) -> Option<()> {
        unsafe {
            let am = &mut renderer.data.lock().asset_manager;
            let vertex0 = am.vertex_buffers.get(&self.vertex0_buffer)?;
            let vertex1 = am.vertex_buffers.get(&self.vertex1_buffer);
            let color = am.vertex_buffers.get(&self.color_buffer);
            let index = am.index_buffers.get(&self.index_buffer)?;

            let ctx = renderer.gpu.context();
            ctx.IASetIndexBuffer(&index.buffer, DXGI_FORMAT(index.format as _), 0);
            if let Some(vertex1) = vertex1 {
                ctx.IASetVertexBuffers(
                    0,
                    2,
                    Some([Some(vertex0.buffer.clone()), Some(vertex1.buffer.clone())].as_ptr()),
                    Some([vertex0.stride as _, vertex1.stride as _].as_ptr()),
                    Some([0, 0].as_ptr()),
                );
            } else {
                ctx.IASetVertexBuffers(
                    0,
                    1,
                    Some([Some(vertex0.buffer.clone())].as_ptr()),
                    Some([vertex0.stride as _].as_ptr()),
                    Some([0].as_ptr()),
                );
            }

            let color = color.unwrap_or(&renderer.gpu.color0_fallback);
            ctx.VSSetShaderResources(0, Some(&[color.srv.clone()]));
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
    pub hash: TagHash,
    pub subscribed_stages: RenderStageSubscriptions,

    buffers: Vec<ModelBuffers>,
    special_meshes: Vec<SpecialMesh>,
}

impl StaticModel {
    pub fn load(am: &mut AssetManager, hash: TagHash) -> anyhow::Result<Self> {
        let model = package_manager().read_tag_struct::<SStaticMesh>(hash)?;
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

        let mut subscribed_stages = model
            .opaque_meshes
            .mesh_groups
            .iter()
            .fold(RenderStageSubscriptions::empty(), |acc, group| {
                acc | group.render_stage
            });

        let special_meshes = model
            .special_meshes
            .iter()
            .map(|mesh| {
                subscribed_stages |= mesh.render_stage;
                SpecialMesh {
                    mesh: mesh.clone(),
                    buffers: ModelBuffers {
                        vertex0_buffer: am.get_or_load_vertex_buffer(mesh.vertex0_buffer),
                        vertex1_buffer: am.get_or_load_vertex_buffer(mesh.vertex1_buffer),
                        color_buffer: am.get_or_load_vertex_buffer(mesh.color_buffer),
                        index_buffer: am.get_or_load_index_buffer(mesh.index_buffer),
                    },
                    technique: am.get_or_load_technique(mesh.technique),
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

    /// ⚠ Expects the `instances` scope to be bound
    pub fn draw(&self, renderer: &Renderer, render_stage: TfxRenderStage, instances_count: u32) {
        if !self.subscribed_stages.is_subscribed(render_stage) {
            return;
        }

        gpu_event!(
            renderer.gpu,
            format!("static_model {}", self.hash.prepend_package_name())
        );

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
            if buffers.bind(renderer).is_none() {
                continue;
            }

            if let Some(technique) = renderer.get_technique_shared(&self.materials[i]) {
                technique.bind(renderer).expect("Failed to bind technique");
            } else {
                continue;
            }

            renderer
                .gpu
                .set_input_layout(group.input_layout_index as usize);
            renderer.gpu.set_input_topology(part.primitive_type);

            unsafe {
                renderer.gpu.context().DrawIndexedInstanced(
                    part.index_count,
                    instances_count,
                    part.index_start,
                    0,
                    0,
                );
            }
        }

        self.draw_special_meshes(renderer, render_stage, instances_count);
    }

    fn draw_special_meshes(
        &self,
        renderer: &Renderer,
        render_stage: TfxRenderStage,
        instances_count: u32,
    ) {
        profiling::scope!("StaticModel::draw_special_meshes");
        for mesh in self
            .special_meshes
            .iter()
            .filter(|m| m.mesh.render_stage == render_stage && m.mesh.lod.is_highest_detail())
        {
            if mesh.buffers.bind(renderer).is_none() {
                continue;
            }

            if let Some(technique) = renderer.get_technique_shared(&mesh.technique) {
                technique.bind(renderer).expect("Failed to bind technique");
                // } else {
                //     continue;
            }

            renderer
                .gpu
                .set_input_layout(mesh.mesh.input_layout_index as usize);
            renderer.gpu.set_input_topology(mesh.mesh.primitive_type);

            unsafe {
                renderer.gpu.context().DrawIndexedInstanced(
                    mesh.mesh.index_count,
                    instances_count,
                    mesh.mesh.index_start,
                    0,
                    0,
                );
            }
        }
    }
}

// TODO(cohae): With children separated into it's own component we can probably merge singular and instances staticmodels
/// Singular static model
pub struct StaticModelSingle {
    pub model: StaticModel,
    pub cbuffer: ConstantBuffer<u8>,
}

impl StaticModelSingle {
    pub fn load(
        gctx: SharedGpuContext,
        am: &mut AssetManager,
        tag: TagHash,
    ) -> anyhow::Result<Self> {
        let model = StaticModel::load(am, tag)?;
        let cbuffer = ConstantBuffer::create_array_init(gctx, &[0u8; 32 + 64])?;
        Ok(Self { model, cbuffer })
    }

    pub fn update_cbuffer(&self, transform: &Transform) {
        profiling::scope!("StaticInstances::update_cbuffer");
        let mat = transform.local_to_world().transpose();
        let mat = Mat4::from_cols(
            mat.x_axis.truncate().extend(transform.translation.x),
            mat.y_axis.truncate().extend(transform.translation.y),
            mat.z_axis.truncate().extend(transform.translation.z),
            mat.w_axis,
        );

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
                        transforms: vec![mat],
                    }
                    .write()
                    .as_slice(),
                )
                .unwrap()
        }
    }
}

/// Parent of all static instances for a model
pub struct StaticInstances {
    pub model: StaticModel,
    pub cbuffer: ConstantBuffer<u8>,
    cbuffer_dirty: bool,
}

impl StaticInstances {
    pub fn new(gpu: Arc<GpuContext>, model: StaticModel, instances: usize) -> anyhow::Result<Self> {
        let cbuffer = ConstantBuffer::create_array_init(gpu, &vec![0u8; 32 + 64 * instances])?;

        Ok(Self {
            model,
            cbuffer,
            cbuffer_dirty: true,
        })
    }

    pub fn mark_dirty(&mut self) {
        self.cbuffer_dirty = true;
    }

    pub fn update_cbuffer(&self, scene: &Scene, instances: &[Entity]) {
        profiling::scope!("StaticInstances::update_cbuffer");
        let mut transforms = Vec::with_capacity(instances.len());
        for &instance in instances {
            if let Ok(transform) = scene.get::<&Transform>(instance) {
                let mat = transform.local_to_world().transpose();
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
                .unwrap();
        }
    }
}

/// A single instance of a static model, can be manipulated individually
/// Rendered by [`StaticInstances`]
pub struct StaticInstance;

pub fn draw_static_instances_system(
    renderer: &Renderer,
    scene: &Scene,
    render_stage: TfxRenderStage,
) {
    profiling::scope!(
        "draw_static_instances_system",
        &format!("render_stage={render_stage:?}")
    );
    for (_, (instances, children)) in scene
        .query::<(&StaticInstances, &Children)>()
        .without::<&Hidden>()
        .iter()
    {
        // TODO(cohae): We want to pull the slot number from the `instances` scope
        instances.cbuffer.bind(1, TfxShaderStage::Vertex);
        instances
            .model
            .draw(renderer, render_stage, children.len() as u32);
    }
    for (_, instances) in scene
        .query::<&StaticModelSingle>()
        .without::<&Hidden>()
        .iter()
    {
        // TODO(cohae): We want to pull the slot number from the `instances` scope
        instances.cbuffer.bind(1, TfxShaderStage::Vertex);
        instances.model.draw(renderer, render_stage, 1);
    }
}

pub fn update_static_instances_system(scene: &Scene) {
    profiling::scope!("update_static_instances_system");
    for (_, (instances, children)) in scene.query::<(&mut StaticInstances, &Children)>().iter() {
        if instances.cbuffer_dirty {
            instances.update_cbuffer(scene, children);
            instances.cbuffer_dirty = false;
        }
    }

    for (_, (transform, model)) in scene.query::<(&Transform, &StaticModelSingle)>().iter() {
        model.update_cbuffer(transform);
    }
}
