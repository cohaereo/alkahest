use alkahest_data::tfx::TfxRenderStage;

use crate::{
    ecs::{
        dynamic_geometry::draw_dynamic_model_system, static_geometry::draw_static_instances_system,
        terrain::draw_terrain_patches_system, Scene,
    },
    gpu_event,
    renderer::Renderer,
};

impl Renderer {
    pub(super) fn run_renderstage_systems(&self, scene: &Scene, stage: TfxRenderStage) {
        gpu_event!(self.gpu, stage.to_string());

        if matches!(
            stage,
            TfxRenderStage::GenerateGbuffer
                | TfxRenderStage::ShadowGenerate
                | TfxRenderStage::DepthPrepass
        ) {
            draw_terrain_patches_system(self, scene);
        }

        if stage == TfxRenderStage::Transparents {
            draw_dynamic_model_system(self, scene, stage);
            draw_static_instances_system(self, scene, stage);
        } else {
            draw_static_instances_system(self, scene, stage);
            draw_dynamic_model_system(self, scene, stage);
        }
    }
}
