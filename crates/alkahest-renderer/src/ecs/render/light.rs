use std::sync::Arc;

use alkahest_data::{
    geometry::EPrimitiveType,
    map::{SLight, SShadowingLight},
    tfx::TfxShaderStage,
};
use anyhow::Context;
use bevy_ecs::{change_detection::DetectChanges, component::Component, system::Query, world::Ref};
use genmesh::{
    generators::{IndexedPolygon, SharedVertex},
    Triangulate,
};
use glam::{Mat4, UVec2, Vec3, Vec4, Vec4Swizzles};
use windows::Win32::Graphics::{
    Direct3D11::{
        ID3D11Buffer, ID3D11DepthStencilState, D3D11_BIND_INDEX_BUFFER, D3D11_BIND_VERTEX_BUFFER,
        D3D11_BUFFER_DESC, D3D11_CLEAR_DEPTH, D3D11_CLEAR_STENCIL, D3D11_COMPARISON_ALWAYS,
        D3D11_DEPTH_STENCILOP_DESC, D3D11_DEPTH_STENCIL_DESC, D3D11_DEPTH_WRITE_MASK_ZERO,
        D3D11_STENCIL_OP_DECR, D3D11_STENCIL_OP_INCR, D3D11_STENCIL_OP_KEEP,
        D3D11_SUBRESOURCE_DATA, D3D11_USAGE_IMMUTABLE,
    },
    Dxgi::Common::DXGI_FORMAT_R16_UINT,
};

use crate::{
    camera::{CameraProjection, Viewport},
    ecs::{
        culling::Frustum,
        transform::Transform,
        visibility::{ViewVisibility, VisibilityHelper},
        Scene,
    },
    gpu::GpuContext,
    gpu_event,
    handle::Handle,
    icons::{ICON_LIGHTBULB_FLUORESCENT_TUBE, ICON_LIGHTBULB_ON, ICON_SPOTLIGHT_BEAM},
    loaders::AssetManager,
    renderer::{gbuffer::ShadowDepthMap, Renderer, ShadowQuality},
    tfx::{
        externs::{self, TextureView},
        technique::Technique,
        view::{RenderStageSubscriptions, View},
    },
};

#[derive(Component)]
pub struct LightRenderer {
    pub projection_matrix: Mat4,

    depth_state: ID3D11DepthStencilState,
    vb_cube: ID3D11Buffer,
    ib_cube: ID3D11Buffer,
    cube_index_count: u32,

    // TfxRenderStage::LightingApply
    technique_shading: Handle<Technique>,
    technique_shading_shadowing: Option<Handle<Technique>>,
    // TfxRenderStage::Volumetrics
    _technique_volumetrics: Handle<Technique>,
    _technique_volumetrics_shadowing: Option<Handle<Technique>>,
    // TfxRenderStage::LightProbeApply
    _technique_compute_lightprobe: Handle<Technique>,
    _technique_compute_lightprobe_shadowing: Option<Handle<Technique>>,

    pub debug_label: String,
    pub debug_info: String,
}

impl LightRenderer {
    pub fn new_empty(gctx: Arc<GpuContext>) -> anyhow::Result<Self> {
        let mesh = genmesh::generators::Cube::new();
        let vertices: Vec<[f32; 3]> = mesh
            .shared_vertex_iter()
            .map(|v| {
                let v = <[f32; 3]>::from(v.pos);
                [v[0], v[1], v[2]]
            })
            .collect();
        let mut indices = vec![];
        for i in mesh.indexed_polygon_iter().triangulate() {
            indices.extend_from_slice(&[i.x as u16, i.y as u16, i.z as u16]);
        }

        let mut ib_cube = None;
        unsafe {
            gctx.device
                .CreateBuffer(
                    &D3D11_BUFFER_DESC {
                        ByteWidth: (indices.len() * 2) as _,
                        Usage: D3D11_USAGE_IMMUTABLE,
                        BindFlags: D3D11_BIND_INDEX_BUFFER.0 as u32,
                        ..Default::default()
                    },
                    Some(&D3D11_SUBRESOURCE_DATA {
                        pSysMem: indices.as_ptr() as _,
                        ..Default::default()
                    }),
                    Some(&mut ib_cube),
                )
                .context("Failed to create index buffer")?
        };

        let mut vb_cube = None;
        unsafe {
            gctx.device
                .CreateBuffer(
                    &D3D11_BUFFER_DESC {
                        ByteWidth: (vertices.len() * 12) as _,
                        Usage: D3D11_USAGE_IMMUTABLE,
                        BindFlags: D3D11_BIND_VERTEX_BUFFER.0 as u32,
                        ..Default::default()
                    },
                    Some(&D3D11_SUBRESOURCE_DATA {
                        pSysMem: vertices.as_ptr() as _,
                        ..Default::default()
                    }),
                    Some(&mut vb_cube),
                )
                .context("Failed to create vertex buffer")?
        };

        let mut depth_state = None;
        unsafe {
            gctx.device
                .CreateDepthStencilState(
                    &D3D11_DEPTH_STENCIL_DESC {
                        DepthEnable: false.into(),
                        DepthWriteMask: D3D11_DEPTH_WRITE_MASK_ZERO,
                        DepthFunc: D3D11_COMPARISON_ALWAYS,
                        StencilEnable: false.into(),
                        StencilReadMask: 0xff,
                        StencilWriteMask: 0xff,
                        FrontFace: D3D11_DEPTH_STENCILOP_DESC {
                            StencilFailOp: D3D11_STENCIL_OP_KEEP,
                            StencilDepthFailOp: D3D11_STENCIL_OP_INCR,
                            StencilPassOp: D3D11_STENCIL_OP_KEEP,
                            StencilFunc: D3D11_COMPARISON_ALWAYS,
                        },
                        BackFace: D3D11_DEPTH_STENCILOP_DESC {
                            StencilFailOp: D3D11_STENCIL_OP_KEEP,
                            StencilDepthFailOp: D3D11_STENCIL_OP_DECR,
                            StencilPassOp: D3D11_STENCIL_OP_KEEP,
                            StencilFunc: D3D11_COMPARISON_ALWAYS,
                        },
                    },
                    Some(&mut depth_state),
                )
                .context("Failed to create light renderer depth state")?
        };

        Ok(Self {
            projection_matrix: Mat4::IDENTITY,
            depth_state: depth_state.unwrap(),
            vb_cube: vb_cube.unwrap(),
            ib_cube: ib_cube.unwrap(),
            cube_index_count: indices.len() as _,

            technique_shading: Handle::none(),
            technique_shading_shadowing: None,
            _technique_volumetrics: Handle::none(),
            _technique_volumetrics_shadowing: None,
            _technique_compute_lightprobe: Handle::none(),
            _technique_compute_lightprobe_shadowing: None,
            debug_label: "Unknown DeferredLight".to_string(),
            debug_info: "Unknown DeferredLight".to_string(),
        })
    }

    pub fn load(
        gctx: Arc<GpuContext>,
        asset_manager: &mut AssetManager,
        light: &SLight,
        debug_label: String,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            projection_matrix: light.light_space_transform,
            technique_shading: asset_manager.get_or_load_technique(light.technique_shading),
            _technique_volumetrics: asset_manager
                .get_or_load_technique(light.technique_volumetrics),
            _technique_compute_lightprobe: asset_manager
                .get_or_load_technique(light.technique_compute_lightprobe),
            debug_label,
            debug_info: format!("{light:X?}"),
            ..Self::new_empty(gctx.clone())?
        })
    }

    pub fn load_shadowing(
        gctx: Arc<GpuContext>,
        asset_manager: &mut AssetManager,
        light: &SShadowingLight,
        debug_label: String,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            projection_matrix: light.light_space_transform,
            technique_shading: asset_manager.get_or_load_technique(light.technique_shading),
            technique_shading_shadowing: Some(
                asset_manager.get_or_load_technique(light.technique_shading_shadowing),
            ),
            _technique_volumetrics: asset_manager
                .get_or_load_technique(light.technique_volumetrics),
            _technique_volumetrics_shadowing: Some(
                asset_manager.get_or_load_technique(light.technique_volumetrics_shadowing),
            ),
            _technique_compute_lightprobe: asset_manager
                .get_or_load_technique(light.technique_compute_lightprobe),
            _technique_compute_lightprobe_shadowing: Some(
                asset_manager.get_or_load_technique(light.technique_compute_lightprobe_shadowing),
            ),
            debug_label,
            debug_info: format!("{light:X?}"),
            ..Self::new_empty(gctx.clone())?
        })
    }

    fn draw(&self, renderer: &Renderer, draw_shadows: bool) {
        gpu_event!(renderer.gpu, &self.debug_label);
        unsafe {
            renderer
                .gpu
                .lock_context()
                .OMSetDepthStencilState(Some(&self.depth_state), 0);

            // Layout 1
            //  - float3 v0 : POSITION0, // Format DXGI_FORMAT_R32G32B32_FLOAT size 12
            renderer.gpu.set_input_layout(1);
            renderer.gpu.set_blend_state(8);
            renderer.gpu.lock_context().IASetVertexBuffers(
                0,
                1,
                Some([Some(self.vb_cube.clone())].as_ptr()),
                Some([12].as_ptr()),
                Some(&0),
            );
            if let Some(tech) = renderer.get_technique_shared(if draw_shadows {
                self.technique_shading_shadowing
                    .as_ref()
                    .unwrap_or(&self.technique_shading)
            } else {
                &self.technique_shading
            }) {
                tech.bind(renderer).expect("Failed to bind technique");
            } else {
                return;
            }

            renderer.gpu.lock_context().IASetIndexBuffer(
                Some(&self.ib_cube),
                DXGI_FORMAT_R16_UINT,
                0,
            );

            renderer.gpu.set_input_topology(EPrimitiveType::Triangles);

            renderer
                .gpu
                .lock_context()
                .DrawIndexed(self.cube_index_count, 0, 0);
        }
    }
}

pub fn draw_light_system(renderer: &Renderer, scene: &mut Scene) {
    profiling::scope!("draw_light_system");
    for (transform, light_renderer, light, vis) in scene
        .query::<(&Transform, &LightRenderer, &SLight, Option<&ViewVisibility>)>()
        .iter(scene)
    {
        if !vis.is_visible(renderer.active_view) {
            continue;
        }

        {
            let externs = &mut renderer.data.lock().externs;
            let Some(view) = &externs.view else {
                error!("No view extern bound for light rendering");
                return;
            };

            let local_to_world_scaled = transform.local_to_world() * light.light_space_transform;
            let view_translation_inverse_mat4 = Mat4::from_translation(-view.position.xyz());
            let local_to_world_relative =
                view_translation_inverse_mat4 * transform.local_to_world();

            let (min, max) = compute_light_bounds(light.light_space_transform);
            let light_local_to_world =
                compute_light_local_to_world(transform.local_to_world(), min, max);

            let existing_deflight = externs.deferred_light.as_ref().cloned().unwrap_or_default();
            externs.deferred_light = Some(externs::DeferredLight {
                unk40: (view_translation_inverse_mat4 * light_local_to_world).inverse(),
                unk80: local_to_world_relative,

                unk100: light.unk50,

                ..existing_deflight
            });

            externs.simple_geometry = Some(externs::SimpleGeometry {
                transform: view.world_to_projective * local_to_world_scaled,
            });
        }

        light_renderer.draw(renderer, false);
    }

    for (transform, light_renderer, light, shadowmap, vis) in scene
        .query::<(
            &Transform,
            &LightRenderer,
            &SShadowingLight,
            Option<&ShadowMapRenderer>,
            Option<&ViewVisibility>,
        )>()
        .iter(scene)
    {
        if !vis.is_visible(renderer.active_view) {
            continue;
        }

        {
            let externs = &mut renderer.data.lock().externs;
            let Some(view) = &externs.view else {
                error!("No view extern bound for light rendering");
                return;
            };
            let transform_relative = Transform {
                translation: transform.translation - view.position.xyz(),
                // translation: Vec3::ZERO,
                ..*transform
            };

            let local_to_world_scaled = transform.local_to_world() * light.light_space_transform;
            let view_translation_inverse_mat4 = Mat4::from_translation(-view.position.xyz());
            let local_to_world_relative =
                view_translation_inverse_mat4 * transform.local_to_world();

            let (min, max) = compute_light_bounds(light.light_space_transform);
            let light_local_to_world =
                compute_light_local_to_world(transform.local_to_world(), min, max);

            let existing_deflight = externs.deferred_light.as_ref().cloned().unwrap_or_default();
            externs.deferred_light = Some(externs::DeferredLight {
                unk40: (view_translation_inverse_mat4 * light_local_to_world).inverse(),
                unk80: local_to_world_relative,

                unk100: light.unk50,
                // unk110: 1.0,
                // unk114: 2000.0,
                // unk118: 1.0,
                // unk11c: 1.0,
                // unk120: 1.0,
                ..existing_deflight
            });

            externs.simple_geometry = Some(externs::SimpleGeometry {
                transform: view.world_to_projective * local_to_world_scaled,
            });

            if let Some(shadowmap) = shadowmap {
                // TODO(cohae): Unknown what this texture is supposed to be. VS loads the first pixel and uses it as multiplier for the shadowmap UVs
                renderer
                    .gpu
                    .shadowmap_vs_t2
                    .bind(&renderer.gpu, 2, TfxShaderStage::Vertex);
                let existing_shadowmap = externs
                    .deferred_shadow
                    .as_ref()
                    .cloned()
                    .unwrap_or_default();
                externs.deferred_shadow = Some(externs::DeferredShadow {
                    unk00: TextureView::RawSRV(shadowmap.depth.texture_view.clone()),
                    resolution_width: shadowmap.resolution() as f32,
                    resolution_height: shadowmap.resolution() as f32,
                    unkc0: shadowmap.camera_to_projective * transform_relative.view_matrix(),
                    unk180: renderer.settings.shadow_quality.pcf_samples() as u8 as f32,
                    ..existing_shadowmap
                })
            }
        }

        light_renderer.draw(
            renderer,
            shadowmap.is_some() && renderer.settings.shadow_quality != ShadowQuality::Off,
        );
    }
}

#[derive(Component)]
pub struct ShadowMapRenderer {
    pub last_update: usize,
    pub stationary_needs_update: bool,

    resolution: u32,
    depth_stationary: ShadowDepthMap,
    depth: ShadowDepthMap,
    viewport: Viewport,
    projection: CameraProjection,
    transform: Transform,

    world_to_camera: Mat4,
    camera_to_projective: Mat4,
}

/// What geometry to render shadows for
#[derive(PartialEq)]
pub enum ShadowGenerationMode {
    /// Only render stationary static geometry. Will clear the stationary depth buffer
    StationaryOnly,

    /// Only render dynamic geometry/animated static geometry. Will copy the depth buffer from the stationary pass to the main depth buffer
    MovingOnly,
    // /// Render both stationary and dynamic geometry. Clears the main depth buffer
    // Both,
}

impl ShadowMapRenderer {
    pub fn new(
        gpu: &GpuContext,
        transform: Transform,
        projection: CameraProjection,
        resolution: u32,
    ) -> anyhow::Result<Self> {
        let depth = ShadowDepthMap::create((resolution, resolution), 1, &gpu.device)?;
        let depth_stationary = ShadowDepthMap::create((resolution, resolution), 1, &gpu.device)?;

        let viewport = Viewport {
            origin: UVec2::ZERO,
            size: UVec2::splat(resolution),
        };

        let world_to_camera = transform.view_matrix();
        let camera_to_projective = projection.matrix(viewport.aspect_ratio());

        Ok(Self {
            last_update: 0,
            stationary_needs_update: true,
            resolution,
            depth_stationary,
            depth,
            projection,
            transform,
            viewport,
            world_to_camera,
            camera_to_projective,
        })
    }

    pub fn resolution(&self) -> u32 {
        self.resolution
    }

    pub fn resize(&mut self, gpu: &GpuContext, resolution: u32) {
        *self = Self::new(gpu, self.transform, self.projection.clone(), resolution).unwrap();
    }

    /// Binds the shadowmap
    pub fn bind_for_generation(
        &mut self,
        transform: &Transform,
        renderer: &Renderer,
        mode: ShadowGenerationMode,
    ) {
        self.world_to_camera = transform.view_matrix();
        self.transform = transform.clone();

        unsafe {
            let view = match mode {
                ShadowGenerationMode::StationaryOnly => {
                    renderer.gpu.lock_context().ClearDepthStencilView(
                        &self.depth_stationary.views[0],
                        (D3D11_CLEAR_DEPTH | D3D11_CLEAR_STENCIL).0 as _,
                        1.0,
                        0,
                    );
                    self.stationary_needs_update = false;
                    &self.depth_stationary.views[0]
                }
                ShadowGenerationMode::MovingOnly => {
                    renderer
                        .gpu
                        .copy_texture(&self.depth_stationary.texture, &self.depth.texture);

                    &self.depth.views[0]
                }
            };

            renderer.gpu.lock_context().OMSetRenderTargets(None, view);
        }
    }
}

impl View for ShadowMapRenderer {
    fn viewport(&self) -> Viewport {
        self.viewport.clone()
    }

    fn subscribed_views(&self) -> RenderStageSubscriptions {
        RenderStageSubscriptions::SHADOW_GENERATE
    }

    fn name(&self) -> String {
        "ShadowRenderer".to_string()
    }

    fn update_extern(&self, x: &mut externs::View) {
        x.world_to_camera = self.world_to_camera;
        x.camera_to_projective = self.camera_to_projective;

        x.derive_matrices(&self.viewport);

        // Only known values are (0, 1, 0, 0) and (0, 3.428143, 0, 0)
        x.view_miscellaneous = Vec4::new(0., 1., 0., 0.);
    }

    fn frustum(&self) -> crate::ecs::culling::Frustum {
        Frustum::from_matrix(self.camera_to_projective * self.world_to_camera)
    }
}

pub enum LightShape {
    Omni,
    Spot,
    Line,
}

impl LightShape {
    /// Discern the light shape from the volume matrix (volume space -> world)
    pub fn from_volume_matrix(m: Mat4) -> LightShape {
        if m.x_axis.x.abs() == 0.0 {
            LightShape::Spot
        } else if m.x_axis.x.abs() != m.y_axis.y.abs() || m.y_axis.y.abs() != m.z_axis.z.abs() {
            LightShape::Line
        } else {
            LightShape::Omni
        }
    }

    pub fn icon(&self) -> char {
        match self {
            LightShape::Omni => ICON_LIGHTBULB_ON,
            LightShape::Spot => ICON_SPOTLIGHT_BEAM,
            LightShape::Line => ICON_LIGHTBULB_FLUORESCENT_TUBE,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            LightShape::Omni => "Omni",
            LightShape::Spot => "Spot",
            LightShape::Line => "Line",
        }
    }
}

pub fn update_shadowrenderer_system(
    mut q_shadowrenderer: Query<(Ref<Transform>, &mut ShadowMapRenderer)>,
) {
    profiling::scope!("update_shadowrenderer_system");
    for (transform, mut shadow) in q_shadowrenderer.iter_mut() {
        if transform.is_changed() {
            shadow.stationary_needs_update = true;
        }
    }
}

fn compute_light_bounds(light_space_transform: Mat4) -> (Vec3, Vec3) {
    let mut points = [
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(-1.0, 1.0, 1.0),
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(1.0, 1.0, -1.0),
        Vec3::new(1.0, 1.0, 1.0),
    ];

    for point in &mut points {
        let p = light_space_transform.mul_vec4(point.extend(1.0));
        let point_w_abs = (-p.wwww()).abs();
        *point = Vec4::select(
            point_w_abs.cmpge(Vec4::splat(0.0001)),
            p / p.wwww(),
            Vec4::W,
        )
        .truncate();
    }

    points
        .iter()
        .fold((Vec3::MAX, Vec3::MIN), |(min, max), &point| {
            (min.min(point), max.max(point))
        })
}

fn compute_light_local_to_world(node_local_to_world: Mat4, min: Vec3, max: Vec3) -> Mat4 {
    let bounds_center = min.midpoint(max);
    let bounds_half_extents = (max - min) / 2.0;

    // First matrix operation ("mat"):
    // Each column is computed by scaling one of node_local_to_world’s axes by the corresponding component of bounds_half_extents,
    // except for the w-axis which is a linear combination of the x, y, and z axes plus the original w-axis.
    let mat = Mat4 {
        x_axis: node_local_to_world.x_axis * bounds_half_extents.x,
        y_axis: node_local_to_world.y_axis * bounds_half_extents.y,
        z_axis: node_local_to_world.z_axis * bounds_half_extents.z,
        w_axis: node_local_to_world.x_axis * bounds_center.x
            + node_local_to_world.y_axis * bounds_center.y
            + node_local_to_world.z_axis * bounds_center.z
            + node_local_to_world.w_axis,
    };

    // Second matrix operation ("mat_scaled"):
    // Scale the x, y, and z axes by 2, and subtract all three from the w-axis.
    let mat_scaled = Mat4 {
        x_axis: mat.x_axis * 2.0,
        y_axis: mat.y_axis * 2.0,
        z_axis: mat.z_axis * 2.0,
        w_axis: mat.w_axis - mat.x_axis - mat.y_axis - mat.z_axis,
    };

    // Third matrix operation (computing light_local_to_world):
    // Rearrange the columns of mat_scaled: swap the x and z axes, leaving y and w unchanged.
    let light_local_to_world = Mat4 {
        x_axis: mat_scaled.z_axis,
        y_axis: mat_scaled.y_axis,
        z_axis: mat_scaled.x_axis,
        w_axis: mat_scaled.w_axis,
    };

    light_local_to_world
}
