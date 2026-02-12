use std::sync::Arc;

use alkahest_render::{
    Renderer,
    camera::CameraProjection,
    cmd_event_span,
    gpu::command_list::CommandList,
    tfx::view::{View, ViewKind},
};
use glam::Mat4;

use crate::world::transform::Transform;

pub struct ShadowMap {
    pub finished_rendering: bool,
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

pub fn s_extract_all_shadowmaps(world: &hecs::World, renderer: &Arc<Renderer>) {
    if !renderer.asset_manager.is_idle() {
        return;
    }

    profiling::scope!("extract_shadowmaps");

    let mut i = 0;
    for (_entity, (shadowmap, view)) in world.query::<(&mut ShadowMap, &mut View)>().iter() {
        if shadowmap.finished_rendering {
            continue;
        }

        if View::FIRST_SHADOW + i >= View::MAX_VIEWS {
            if let ViewKind::Shadow(v) = &mut view.kind {
                v.index = usize::MAX;
            };

            continue;
        }

        view.update(
            shadowmap.world_to_camera,
            shadowmap.camera_to_projective,
            view.resolution(),
        );

        let ViewKind::Shadow(v) = &mut view.kind else {
            continue;
        };

        v.index = View::FIRST_SHADOW + i;
        debug_assert!(v.index >= View::FIRST_SHADOW);
        i += 1;

        renderer.cull_view(v.index, view);
    }
}

pub fn s_submit_all_shadowmaps(
    world: &hecs::World,
    cmd: &mut CommandList,
    renderer: &Arc<Renderer>,
) {
    if !renderer.asset_manager.is_idle() {
        return;
    }

    profiling::scope!("render_shadowmaps");
    let _gpuspan = renderer.profiler.scope(cmd, "render_shadowmaps");

    for (_entity, (shadowmap, view)) in world.query::<(&mut ShadowMap, &View)>().iter() {
        if shadowmap.finished_rendering {
            continue;
        }

        let ViewKind::Shadow(v) = &view.kind else {
            continue;
        };

        if v.index >= View::MAX_VIEWS {
            continue;
        }

        {
            cmd_event_span!(cmd, format!("prepare_view_{}", view.name));
            let _gpuspan = renderer
                .profiler
                .scope(cmd, format!("prepare_view_{}", view.name));

            for node in renderer.frame_packet.read().iter_visible(v.index) {
                if let Some(render_object) = renderer
                    .objects
                    .write()
                    .get_mut(node.render_object_handle.into())
                {
                    render_object.prepare(renderer, v.index, &*node.data);
                } else if node.render_object_handle.is_valid() {
                    error!("Render object not found: {:?}", node.render_object_handle);
                }
            }
        }

        renderer.submit_view(cmd, view, None);

        shadowmap.finished_rendering = true;
    }
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
