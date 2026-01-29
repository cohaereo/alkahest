use std::sync::Arc;

use alkahest_render::{Renderer, camera::CameraProjection, tfx::view::View};
use glam::Mat4;
use parking_lot::Mutex;

use crate::world::transform::Transform;

pub struct ShadowMap {
    pub finished_rendering: bool,
    // pub surface: Arc<Mutex<Option<Surface>>>,
    pub world_to_camera: Mat4,
    pub camera_to_projective: Mat4,
}

impl ShadowMap {
    pub fn create(transform: Transform, fov: f32, near: f32, far: f32) -> Self {
        let world_to_camera = transform.view_matrix();
        let projection = CameraProjection::Perspective.matrix(1.0, fov, near, far);

        ShadowMap {
            // view: Arc::new(Mutex::new(view)),
            finished_rendering: false,
            // surface: Arc::new(Mutex::new(None)),
            world_to_camera,
            camera_to_projective: projection,
        }
    }

    // fn initialize_surface(&mut self, gpu: &Gpu) {
    //     let surface_desc = SurfaceDesc::builder("shadowmap", SizeRelativity::Absolute)
    //         .format(dxgi::Format::R32Typeless)
    //         .depth_format(dxgi::Format::D32Float)
    //         .view_format(dxgi::Format::R32Float)
    //         .build();
    //     let surface = Surface::new(
    //         &gpu.device,
    //         (Self::SHADOWMAP_RESOLUTION, Self::SHADOWMAP_RESOLUTION),
    //         surface_desc,
    //     )
    //     .expect("Failed to create shadowmap surface");

    //     *self.surface.lock() = Some(surface);
    // }

    // pub fn bind(&mut self, cmd: &mut CommandList, renderer: &Renderer) {
    //     if self.surface.lock().is_none() {
    //         self.initialize_surface(cmd.gpu());
    //     }

    //     let surface_lock = self.surface.lock();
    //     let shadow_surface = surface_lock
    //         .as_ref()
    //         .expect("unreachable: shadow surface was just initialized");
    //     shadow_surface.clear_depth(cmd, 0.0, 0);
    //     shadow_surface.bind_single(cmd);

    //     let ext = renderer.externs.get_mut();
    //     ext.view.update(
    //         self.world_to_camera,
    //         self.camera_to_projective,
    //         (Self::SHADOWMAP_RESOLUTION, Self::SHADOWMAP_RESOLUTION),
    //     );
    //     renderer.globals.scopes.view.bind(cmd).unwrap();
    // }
}

pub fn s_wants_to_render_shadowmaps(world: &hecs::World) -> bool {
    for (_entity, shadowmap) in world.query::<&ShadowMap>().iter() {
        if !shadowmap.finished_rendering {
            return true;
        }
    }
    false
}

// pub fn s_render_all_shadowmaps(
//     world: &hecs::World,
//     cmd: &mut CommandList,
//     renderer: &Arc<Renderer>,
// ) {
//     profiling::scope!("render_shadowmaps");
//     let _gpuspan = renderer.profiler.scope(cmd, "render_shadowmaps");
//     if renderer.asset_manager.count_loading() > 0 {
//         return;
//     }

//     cmd.state = PipelineState::new(Some(0), Some(2), Some(2), Some(6));
//     cmd.flush_states();

//     // cmd.set_depth_mode(DepthMode::Forward);
//     for (_entity, shadowmap) in world.query::<&mut ShadowMap>().iter() {
//         renderer
//             .common
//             .shadowmap_vs_t2
//             .bind(cmd, 2, ShaderStage::Vertex);

//         if shadowmap.surface.lock().is_some() {
//             // Shadow map already rendered
//             continue;
//         }

//         shadowmap.bind(cmd, renderer);
//         renderer.submit_stage(
//             cmd,
//             RenderStage::ShadowGenerate,
//             FeatureRendererSubscription::all(),
//         );
//     }
//     // cmd.set_depth_mode(DepthMode::Reverse);
// }
