use crate::Renderer;

impl Renderer {
    pub const NUM_CASCADES: usize = 4;
    pub const MAX_CASCADE_DISTANCE: f32 = 600.0;
    pub const CASCADE_DISTANCES: [f32; Self::NUM_CASCADES] =
        [10.0, 30.0, 100.0, Self::MAX_CASCADE_DISTANCE];

    pub fn get_cascade_distance_range(index: usize) -> (f32, f32) {
        let z_near = if index == 0 {
            0.05
        } else {
            Self::CASCADE_DISTANCES[index - 1]
        };
        let z_far = Self::CASCADE_DISTANCES[index];
        (z_near, z_far)
    }

    // pub fn submit_sun_shadows(
    //     self: &Arc<Self>,
    //     cmd: &mut CommandList,
    //     camera: &Camera,
    //     view: &View,
    // ) {
    //     if !view.settings.sun_shadows {
    //         return;
    //     }

    //     profiling::scope!("submit_sun_shadows");
    //     let _gpuspan = self.profiler.scope(cmd, "submit_sun_shadows");

    //     *self.surfaces.write() = view.surfaces.clone();

    //     // let mut camera = Camera {
    //     //     position: vec3(0.0, 0.0, 20.0),
    //     //     fov_y: 30.0,
    //     //     far: Self::MAX_CASCADE_DISTANCE,
    //     //     ..Default::default()
    //     // };

    //     // camera.update();
    //     // self.immediate.lock().frustum(&camera.frustum, 0xffffffff);

    //     let mut sun_dir = self
    //         .externs
    //         .get_global_channel_by_name("sun_light_direction")
    //         .xyz();
    //     if sun_dir.length() < 0.01 {
    //         sun_dir = Vec3::Z;
    //     }
    //     let sun_dir = -sun_dir.normalize();

    //     cmd.set_depth_mode(DepthMode::Forward);
    //     for c in 0..Self::NUM_CASCADES {
    //         profiling::scope!("submit_sun_shadow_cascade", &format!("cascade {}", c));
    //         let _gpuspan = self.profiler.scope(cmd, format!("shadow_cascade_{c}"));
    //         let shadow_map = view.sun_shadow_map_cascades[c];
    //         self.bind_surfaces(cmd, &[], Some(shadow_map));
    //         self.clear_surface_depth(cmd, shadow_map, 1.0, 0);

    //         let (z_near, z_far) = Self::get_cascade_distance_range(c);

    //         let (world_to_camera, camera_to_projective) =
    //             camera.build_shadow_cascade(sun_dir, z_near, z_far);

    //         self.prepare_externs(cmd, view);
    //         let ext = self.externs.get_mut();
    //         ext.view
    //             .update(world_to_camera, camera_to_projective, (2048, 2048));
    //         {
    //             let e = self.externs.get_mut();
    //             let mat = camera_to_projective * world_to_camera;
    //             view.cascade_matrices.write()[c] = mat;
    //             e.view.world_to_projective = mat;
    //             e.view.camera_to_world = Mat4::ZERO;
    //             // e.view.camera_to_projective = camera_to_projective * world_to_camera;
    //             e.view.camera_to_projective.w_axis = mat.w_axis;
    //             // vec4(0.0, 0.0, 0.1, 1.0);
    //         }
    //         self.globals.scopes.view.bind(cmd).unwrap();

    //         {
    //             cmd_event_span!(cmd, "shadow_generate");
    //             cmd.state = PipelineState::new(Some(0), Some(2), Some(7), Some(0));
    //             self.common
    //                 .shadowmap_vs_t2
    //                 .bind(cmd, 2, ShaderStage::Vertex);
    //             self.submit_stage_parallel_apply(
    //                 cmd,
    //                 RenderStage::ShadowGenerate,
    //                 FeatureRendererSubscription::all(),
    //             );
    //         }
    //     }
    //     cmd.set_depth_mode(DepthMode::Reverse);
    // }
}
