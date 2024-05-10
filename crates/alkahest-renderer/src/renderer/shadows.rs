use std::sync::atomic::Ordering;

use alkahest_data::{
    technique::StateSelection,
    tfx::{TfxRenderStage, TfxShaderStage},
};

use crate::{
    ecs::{
        light::{LightRenderer, ShadowMapRenderer},
        Scene,
    },
    gpu_event,
    renderer::Renderer,
};
use crate::ecs::transform::Transform;

impl Renderer {
    pub fn update_shadow_maps(&self, scene: &Scene) {
        if !self.render_settings.shadows || self.render_settings.matcap {
            return;
        }

        self.gpu
            .use_flipped_depth_comparison
            .store(true, Ordering::Relaxed);

        gpu_event!(self.gpu, "update_shadow_maps");
        for (e, (transform, shadow)) in scene
            .query::<(&Transform, &mut ShadowMapRenderer)>()
            .iter()
        {
            if shadow.update_timer <= 0 {
                gpu_event!(self.gpu, format!("update_shadow_map_{}", e.id()));
                shadow.update_timer = 4;

                shadow.bind_for_generation(transform, self);
                self.bind_view(shadow);

                self.gpu.current_states.store(StateSelection::new(
                    Some(0),
                    Some(2),
                    Some(2),
                    Some(0),
                ));
                self.gpu.flush_states();

                self.gpu
                    .shadowmap_vs_t2
                    .bind(&self.gpu, 2, TfxShaderStage::Vertex);
                self.run_renderstage_systems(scene, TfxRenderStage::ShadowGenerate);
            } else {
                shadow.update_timer -= 1;
            }
        }

        self.gpu
            .use_flipped_depth_comparison
            .store(false, Ordering::Relaxed);
    }
}
