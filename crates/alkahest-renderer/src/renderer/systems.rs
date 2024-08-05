use alkahest_data::tfx::TfxRenderStage;

use crate::{
    ecs::{
        render::{
            static_geometry::draw_static_instances_system, terrain::draw_terrain_patches_system,
        },
        Scene,
    },
    gpu_event,
    renderer::Renderer,
    shader::shader_ball::draw_shaderball_system,
};

impl Renderer {
    pub(super) fn run_renderstage_systems(&self, scene: &mut Scene, stage: TfxRenderStage) {
        gpu_event!(self.gpu, stage.to_string());

        draw_terrain_patches_system(self, scene, stage);
        draw_shaderball_system(self, scene, stage);

        draw_static_instances_system(self, scene, stage);
        // draw_dynamic_model_system(self, scene, stage);
    }
}
