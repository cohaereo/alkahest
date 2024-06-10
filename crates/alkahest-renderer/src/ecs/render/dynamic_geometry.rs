use alkahest_data::{
    entity::{SDynamicMesh, SDynamicMeshPart, SDynamicModel, Unk808072c5},
    tfx::{TfxFeatureRenderer, TfxRenderStage, TfxShaderStage},
};
use alkahest_pm::package_manager;
use anyhow::ensure;
use destiny_pkg::TagHash;
use glam::Vec4;
use itertools::Itertools;
use tiger_parse::PackageManagerExt;

use crate::{
    ecs::{
        common::Hidden,
        render::{decorators::DecoratorRenderer, static_geometry::ModelBuffers},
        transform::Transform,
        Scene,
    },
    gpu::buffer::ConstantBuffer,
    gpu_event,
    handle::Handle,
    loaders::AssetManager,
    renderer::Renderer,
    tfx::{
        externs, scope, scope::ScopeSkinning, technique::Technique, view::RenderStageSubscriptions,
    },
    util::packages::TagHashExt,
};

pub struct DynamicModel {
    mesh_buffers: Vec<ModelBuffers>,

    technique_map: Vec<Unk808072c5>,
    techniques: Vec<Handle<Technique>>,

    pub model: SDynamicModel,
    pub mesh_stages: Vec<RenderStageSubscriptions>,
    pub subscribed_stages: RenderStageSubscriptions,
    part_techniques: Vec<Vec<Handle<Technique>>>,

    pub selected_mesh: usize,
    pub selected_variant: usize,
    variant_count: usize,

    identifier_count: usize,

    pub hash: TagHash,
    pub feature_type: TfxFeatureRenderer,
}

impl DynamicModel {
    pub fn load(
        am: &mut AssetManager,
        hash: TagHash,
        technique_map: Vec<Unk808072c5>,
        techniques: Vec<TagHash>,
        feature_type: TfxFeatureRenderer,
    ) -> anyhow::Result<Self> {
        let model = package_manager().read_tag_struct::<SDynamicModel>(hash)?;
        let techniques = techniques
            .iter()
            .map(|&tag| am.get_or_load_technique(tag))
            .collect_vec();

        let mesh_buffers = model
            .meshes
            .iter()
            .map(|m| ModelBuffers {
                vertex0_buffer: am.get_or_load_vertex_buffer(m.vertex0_buffer),
                vertex1_buffer: am.get_or_load_vertex_buffer(m.vertex1_buffer),
                color_buffer: am.get_or_load_vertex_buffer(m.color_buffer),
                index_buffer: am.get_or_load_index_buffer(m.index_buffer),
            })
            .collect_vec();

        let mesh_stages = model
            .meshes
            .iter()
            .map(|m| RenderStageSubscriptions::from_partrange_list(&m.part_range_per_render_stage))
            .collect_vec();

        let part_techniques = model
            .meshes
            .iter()
            .map(|m| {
                m.parts
                    .iter()
                    .map(|p| am.get_or_load_technique(p.technique))
                    .collect_vec()
            })
            .collect_vec();

        let variant_count = technique_map
            .iter()
            .filter(|m| m.unk8 == 0)
            .map(|m| m.technique_count as usize)
            .next()
            .unwrap_or(0);

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

        Ok(Self {
            selected_variant: 0,
            variant_count,
            selected_mesh: 0,
            identifier_count,
            mesh_buffers,
            technique_map,
            techniques,
            model,
            subscribed_stages: mesh_stages
                .iter()
                .fold(RenderStageSubscriptions::empty(), |acc, &x| acc | x),
            mesh_stages,
            part_techniques,
            hash,
            feature_type,
        })
    }

    pub fn mesh_count(&self) -> usize {
        self.model.meshes.len()
    }

    pub fn variant_count(&self) -> usize {
        self.variant_count
    }

    pub fn identifier_count(&self) -> usize {
        self.identifier_count
    }

    fn get_variant_technique(&self, index: u16, variant: usize) -> Option<Handle<Technique>> {
        if index == u16::MAX {
            None
        } else {
            if let Some(variant_range) = &self.technique_map.get(index as usize) {
                Some(
                    self.techniques[variant_range.technique_start as usize
                        + (variant % variant_range.technique_count as usize)]
                        .clone(),
                )
            } else {
                None
            }
        }
    }

    /// ⚠ Expects the `rigid_model` scope to be bound
    pub fn draw(
        &self,
        renderer: &Renderer,
        render_stage: TfxRenderStage,
        identifier: u16,
    ) -> anyhow::Result<()> {
        self.draw_wrapped(
            renderer,
            render_stage,
            identifier,
            |_, renderer, _mesh, part| unsafe {
                renderer
                    .gpu
                    .context()
                    .DrawIndexed(part.index_count, part.index_start, 0);
            },
        )
    }

    pub fn draw_wrapped<F>(
        &self,
        renderer: &Renderer,
        render_stage: TfxRenderStage,
        identifier: u16,
        f: F,
    ) -> anyhow::Result<()>
    where
        F: Fn(&Self, &Renderer, &SDynamicMesh, &SDynamicMeshPart),
    {
        if !renderer.render_settings.stage_transparent
            && render_stage == TfxRenderStage::Transparents
        {
            return Ok(());
        }

        if !renderer.render_settings.stage_decals && render_stage == TfxRenderStage::Decals {
            return Ok(());
        }

        if !renderer.render_settings.stage_decals_additive
            && render_stage == TfxRenderStage::DecalsAdditive
        {
            return Ok(());
        }

        gpu_event!(
            renderer.gpu,
            format!(
                "{} {}",
                self.feature_type.short(),
                self.hash.prepend_package_name()
            )
        );

        profiling::scope!("DynamicModel::draw", format!("mesh={}", self.selected_mesh));
        // ensure!(self.selected_mesh < self.mesh_count(), "Invalid mesh index");
        ensure!(
            self.selected_variant < self.variant_count() || self.variant_count() == 0,
            "Material variant out of range"
        );

        let mesh = &self.model.meshes[self.selected_mesh];
        let stages = &self.mesh_stages[self.selected_mesh];
        if !stages.is_subscribed(render_stage) {
            return Ok(());
        }

        renderer
            .gpu
            .set_input_layout(mesh.get_input_layout_for_stage(render_stage) as usize);
        self.mesh_buffers[self.selected_mesh].bind(renderer);
        for part_index in mesh.get_range_for_stage(render_stage) {
            let part = &mesh.parts[part_index];
            if identifier != u16::MAX && part.external_identifier != identifier {
                continue;
            }

            if !part.lod_category.is_highest_detail() {
                continue;
            }

            let variant_material =
                self.get_variant_technique(part.variant_shader_index, self.selected_variant);

            if let Some(technique) =
                renderer.get_technique_shared(&self.part_techniques[self.selected_mesh][part_index])
            {
                technique.bind(renderer).expect("Failed to bind technique");
                // } else {
                //     continue;
            }

            if let Some(technique) = variant_material
                .and_then(|t| renderer.data.lock().asset_manager.techniques.get_shared(&t))
            {
                technique
                    .bind(renderer)
                    .expect("Failed to bind variant technique");
            }

            if stages.contains(RenderStageSubscriptions::COMPUTE_SKINNING) {
                unsafe {
                    renderer
                        .gpu
                        .context()
                        .VSSetShader(&renderer.gpu.util_resources.entity_vs_override, None);
                }
            }

            renderer.gpu.set_input_topology(part.primitive_type);

            f(self, renderer, mesh, part);
        }

        Ok(())
    }
}

pub struct DynamicModelComponent {
    pub model: DynamicModel,
    pub ext: externs::RigidModel,
    pub cbuffer: ConstantBuffer<externs::RigidModel>,
    pub cbuffer_skinning: Option<ConstantBuffer<ScopeSkinning>>,
    pub identifier: u16,
    cbuffer_dirty: bool,
}

impl DynamicModelComponent {
    pub fn load(
        renderer: &Renderer,
        transform: &Transform,
        hash: TagHash,
        technique_map: Vec<Unk808072c5>,
        techniques: Vec<TagHash>,
        feature_type: TfxFeatureRenderer,
    ) -> anyhow::Result<Self> {
        let model = DynamicModel::load(
            &mut renderer.data.lock().asset_manager,
            hash,
            technique_map,
            techniques,
            feature_type,
        )?;

        let mut d = Self {
            identifier: u16::MAX,
            cbuffer_skinning: model
                .subscribed_stages
                .contains(RenderStageSubscriptions::COMPUTE_SKINNING)
                .then(|| ConstantBuffer::create(renderer.gpu.clone(), None).unwrap()),
            model,
            ext: Default::default(),
            cbuffer: ConstantBuffer::create(renderer.gpu.clone(), None)?,
            cbuffer_dirty: true,
        };
        d.ext = d.create_extern(transform);
        d.cbuffer = ConstantBuffer::create(renderer.gpu.clone(), Some(&d.ext))?;

        Ok(d)
    }

    pub fn mark_dirty(&mut self) {
        self.cbuffer_dirty = true;
    }

    fn create_extern(&self, transform: &Transform) -> externs::RigidModel {
        externs::RigidModel {
            mesh_to_world: transform.local_to_world(),
            position_scale: self.model.model.model_scale,
            position_offset: self.model.model.model_offset,
            texcoord0_scale_offset: Vec4::new(
                self.model.model.texcoord_scale.x,
                self.model.model.texcoord_scale.y,
                self.model.model.texcoord_offset.x,
                self.model.model.texcoord_offset.y,
            ),
            dynamic_sh_ao_values: Vec4::new(0.0, 0.0, 0.0, 1.0),
        }
    }

    pub(self) fn update_cbuffer(&mut self, transform: &Transform) {
        let ext = self.create_extern(transform);

        self.cbuffer.write(&ext).unwrap();
        if let Some(cbuffer_skinning) = &mut self.cbuffer_skinning {
            cbuffer_skinning
                .write(&ScopeSkinning {
                    offset_scale: transform.translation.extend(transform.scale.x),
                    texcoord0_scale_offset: ext.texcoord0_scale_offset,
                    ..Default::default()
                })
                .unwrap();
        }

        self.ext = ext;
    }

    pub fn draw(&self, renderer: &Renderer, render_stage: TfxRenderStage) -> anyhow::Result<()> {
        // cohae: We're doing this in reverse. Normally we'd write the extern first, then copy that to scope data
        renderer.data.lock().externs.rigid_model = Some(self.ext.clone());

        // if let Some(cbuffer_skinning) = &self.cbuffer_skinning {
        //     cbuffer_skinning.bind(1, TfxShaderStage::Vertex);
        //     cbuffer_skinning.bind(1, TfxShaderStage::Pixel);
        // } else {
        // TODO(cohae): We want to pull the slot number from the `rigid_model` scope
        self.cbuffer.bind(1, TfxShaderStage::Vertex);
        // }

        // TODO(cohae): Error reporting
        self.model.draw(renderer, render_stage, self.identifier)
    }
}

pub fn draw_dynamic_model_system(renderer: &Renderer, scene: &Scene, render_stage: TfxRenderStage) {
    profiling::scope!(
        "draw_dynamic_model_system",
        &format!("render_stage={render_stage:?}")
    );

    let mut entities = Vec::new();
    for (e, dynamic) in scene
        .query::<&DynamicModelComponent>()
        .without::<&Hidden>()
        .iter()
    {
        if !dynamic.model.subscribed_stages.is_subscribed(render_stage) {
            continue;
        }

        if !renderer.render_settings.feature_sky
            && dynamic.model.feature_type == TfxFeatureRenderer::SkyTransparent
        {
            continue;
        }

        if !renderer.render_settings.feature_dynamics
            && matches!(
                dynamic.model.feature_type,
                TfxFeatureRenderer::RigidObject | TfxFeatureRenderer::DynamicObjects
            )
        {
            continue;
        }

        entities.push((e, dynamic.model.feature_type));
    }

    entities.sort_by_key(|(_, feature_type)| match feature_type {
        TfxFeatureRenderer::Water => 0,
        TfxFeatureRenderer::SkyTransparent => 1,
        TfxFeatureRenderer::RigidObject | TfxFeatureRenderer::DynamicObjects => 2,
        _ => 99,
    });

    for (e, _feature_type) in entities {
        let dynamic = scene.get::<&DynamicModelComponent>(e).unwrap();

        renderer.pickbuffer.with_entity(e, || {
            dynamic.draw(renderer, render_stage).unwrap();
        });
    }

    if renderer.render_settings.feature_decorators {
        for (e, decorator) in scene
            .query::<&DecoratorRenderer>()
            .without::<&Hidden>()
            .iter()
        {
            renderer.pickbuffer.with_entity(e, || {
                decorator.draw(renderer, render_stage).unwrap();
            });
        }
    }
}

pub fn update_dynamic_model_system(scene: &Scene) {
    profiling::scope!("update_dynamic_model_system");
    for (_, (transform, model)) in scene
        .query::<(&Transform, &mut DynamicModelComponent)>()
        .iter()
    {
        if model.cbuffer_dirty {
            model.update_cbuffer(transform);
            model.cbuffer_dirty = false;
        }
    }
}
