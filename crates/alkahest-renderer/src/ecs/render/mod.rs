use alkahest_data::tfx::TfxRenderStage;
use hecs::Entity;

use crate::{
    ecs::{
        render::{
            decorators::DecoratorRenderer,
            dynamic_geometry::DynamicModelComponent,
            static_geometry::{StaticInstances, StaticModelSingle},
            terrain::TerrainPatches,
        },
        transform::Transform,
        Scene,
    },
    renderer::Renderer,
    shader::shader_ball::ShaderBallComponent,
};

pub mod decorators;
pub mod dynamic_geometry;
pub mod light;
pub mod static_geometry;
pub mod terrain;

/// Draw a specific entity. Only works for entities with geometry, but not screen-space decals, lights, etc
pub fn draw_entity(scene: &Scene, entity: Entity, renderer: &Renderer, stage: TfxRenderStage) {
    let Ok(er) = scene.entity(entity) else {
        return;
    };

    // Supported renderers: StaticInstances, StaticModelSingle, TerrainPatches, DecoratorRenderer, DynamicModelComponent
    if let Some(static_instances) = er.get::<&StaticInstances>() {
        static_instances.draw(renderer, stage);
    } else if let Some(static_model_single) = er.get::<&StaticModelSingle>() {
        static_model_single.draw(renderer, stage);
    } else if let Some(terrain_patches) = er.get::<&TerrainPatches>() {
        terrain_patches.draw(renderer, stage);
    } else if let Some(decorator_renderer) = er.get::<&DecoratorRenderer>() {
        decorator_renderer.draw(renderer, stage).unwrap();
    } else if let Some(dynamic_model_component) = er.get::<&DynamicModelComponent>() {
        dynamic_model_component.draw(renderer, stage).unwrap();
    } else if let Some((shaderball, transform)) =
        er.query::<(&ShaderBallComponent, &Transform)>().get()
    {
        shaderball.draw(renderer, transform, stage);
    }
}
