use alkahest_data::tfx::{TfxRenderStage, TfxShaderStage};
use bevy_ecs::entity::Entity;

use crate::{
    ecs::{
        hierarchy::Parent,
        render::{
            decorators::DecoratorRenderer,
            dynamic_geometry::DynamicModelComponent,
            static_geometry::{
                create_instances_scope, StaticInstance, StaticInstances, StaticModelSingle,
            },
            terrain::TerrainPatches,
        },
        transform::Transform,
        Scene,
    },
    gpu::buffer::ConstantBuffer,
    renderer::Renderer,
    shader::shader_ball::ShaderBallComponent,
    util::black_magic::EntityRefDarkMagic,
};

pub mod decorators;
pub mod dynamic_geometry;
pub mod havok;
pub mod light;
pub mod static_geometry;
pub mod terrain;

/// Draw a specific entity. Only works for entities with geometry, but not screen-space decals, lights, etc
/// Ignores the renderer's feature visibility settings
pub fn draw_entity(
    scene: &mut Scene,
    entity: Entity,
    renderer: &Renderer,
    single_statics_cb: Option<&ConstantBuffer<u8>>,
    render_stage: TfxRenderStage,
) {
    let Some(er) = scene.get_entity(entity) else {
        return;
    };

    // Supported renderers: StaticInstances, StaticModelSingle, TerrainPatches, DecoratorRenderer, DynamicModelComponent
    if let Some(static_instances) = er.get::<StaticInstances>() {
        static_instances.draw(renderer, render_stage);
    } else if let Some(static_model_single) = er.get::<StaticModelSingle>() {
        static_model_single.draw(renderer, render_stage);
    } else if let Some(terrain_patches) = er.get::<TerrainPatches>() {
        terrain_patches.draw(renderer, render_stage);
    } else if let Some(decorator_renderer) = er.get::<DecoratorRenderer>() {
        decorator_renderer.draw(renderer, render_stage).unwrap();
    } else if let Some(dynamic_model_component) = er.get::<DynamicModelComponent>() {
        dynamic_model_component
            .draw(renderer, render_stage)
            .unwrap();
    } else if let Ok((shaderball, transform)) = scene
        .query::<(&ShaderBallComponent, &Transform)>()
        .get(scene, entity)
    {
        shaderball.draw(renderer, transform, render_stage);
    } else if let Ok((transform, _instance, parent)) = scene
        .query::<(&Transform, &StaticInstance, &Parent)>()
        .get(scene, entity)
    {
        if let Some(model) = scene.get::<StaticInstances>(parent.0) {
            if let Some(cbuffer) = single_statics_cb {
                cbuffer.bind(
                    renderer.render_globals.scopes.chunk_model.vertex_slot() as u32,
                    TfxShaderStage::Vertex,
                );
                unsafe {
                    cbuffer
                        .write_array(
                            create_instances_scope(
                                &model.model.model.opaque_meshes,
                                std::slice::from_ref(transform),
                            )
                            .write()
                            .as_slice(),
                        )
                        .unwrap();
                }
                renderer.pickbuffer.with_entity(entity, || {
                    model.model.draw(renderer, render_stage, 1);
                });
            }
        }
    }
}

pub fn update_entity_transform(scene: &mut Scene, entity: Entity) {
    if let Ok((_static_instances, parent)) = scene
        .query::<(&StaticInstance, &Parent)>()
        .get(scene, entity)
    {
        if let Some(parent) = scene.get_entity(parent.0) {
            if let Some(mut static_instances) = parent.get_mut::<StaticInstances>() {
                static_instances.mark_dirty();
            }
        }
    }

    if let Some(mut dynamic) = scene.get_mut::<DynamicModelComponent>(entity) {
        dynamic.mark_dirty();
    }
}
