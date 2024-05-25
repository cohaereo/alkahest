use std::sync::atomic::Ordering;

use alkahest_data::{
    technique::StateSelection,
    tfx::{TfxRenderStage, TfxShaderStage},
};

use crate::{
    ecs::{render::light::ShadowMapRenderer, transform::Transform, Scene},
    gpu_event,
    renderer::Renderer,
};

impl Renderer {
    pub fn update_shadow_maps(&self, scene: &Scene) {
        if !self.render_settings.shadows || self.render_settings.matcap {
            return;
        }

        self.gpu
            .use_flipped_depth_comparison
            .store(true, Ordering::Relaxed);

        gpu_event!(self.gpu, "update_shadow_maps");
        let mut shadow_renderers = vec![];
        for (e, (_transform, shadow)) in
            scene.query::<(&Transform, &mut ShadowMapRenderer)>().iter()
        {
            shadow_renderers.push((e, shadow.last_update));
        }

        shadow_renderers.sort_by_key(|(_, last_update)| *last_update);
        shadow_renderers.truncate(self.render_settings.shadow_updates_per_frame);

        for (e, _) in shadow_renderers {
            gpu_event!(self.gpu, format!("update_shadow_map_{}", e.id()));

            let er = scene.entity(e).unwrap();
            let mut shadow = er.get::<&mut ShadowMapRenderer>().unwrap();
            shadow.last_update = self.frame_index;
            let transform = er.get::<&Transform>().unwrap();

            shadow.bind_for_generation(&transform, self);
            self.bind_view(&*shadow);

            self.gpu
                .current_states
                .store(StateSelection::new(Some(0), Some(2), Some(2), Some(6)));
            self.gpu.flush_states();

            self.gpu
                .shadowmap_vs_t2
                .bind(&self.gpu, 2, TfxShaderStage::Vertex);
            self.run_renderstage_systems(scene, TfxRenderStage::ShadowGenerate);
        }

        self.gpu
            .use_flipped_depth_comparison
            .store(false, Ordering::Relaxed);
    }
}
