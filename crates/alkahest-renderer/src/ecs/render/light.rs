use alkahest_data::{
    geometry::EPrimitiveType,
    map::{SLight, SShadowingLight},
    tfx::TfxShaderStage,
};
use anyhow::Context;
use genmesh::{
    generators::{IndexedPolygon, SharedVertex},
    Triangulate,
};
use glam::{Mat4, UVec2, Vec4, Vec4Swizzles};
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
    ecs::{common::Hidden, transform::Transform, Scene},
    gpu::{GpuContext, SharedGpuContext},
    gpu_event,
    handle::Handle,
    icons::{ICON_LIGHTBULB_FLUORESCENT_TUBE, ICON_LIGHTBULB_ON, ICON_SPOTLIGHT_BEAM},
    loaders::AssetManager,
    renderer::{gbuffer::ShadowDepthMap, Renderer},
    tfx::{
        externs,
        externs::TextureView,
        technique::Technique,
        view::{RenderStageSubscriptions, View},
    },
};

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
    technique_volumetrics: Handle<Technique>,
    technique_volumetrics_shadowing: Option<Handle<Technique>>,
    // TfxRenderStage::LightProbeApply
    technique_compute_lightprobe: Handle<Technique>,
    technique_compute_lightprobe_shadowing: Option<Handle<Technique>>,

    pub debug_label: String,
    pub debug_info: String,
}

impl LightRenderer {
    pub fn new_empty(gctx: SharedGpuContext) -> anyhow::Result<Self> {
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
            technique_volumetrics: Handle::none(),
            technique_volumetrics_shadowing: None,
            technique_compute_lightprobe: Handle::none(),
            technique_compute_lightprobe_shadowing: None,
            debug_label: "Unknown DeferredLight".to_string(),
            debug_info: "Unknown DeferredLight".to_string(),
        })
    }

    pub fn load(
        gctx: SharedGpuContext,
        asset_manager: &mut AssetManager,
        light: &SLight,
        debug_label: String,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            projection_matrix: light.light_to_world,
            technique_shading: asset_manager.get_or_load_technique(light.technique_shading),
            technique_volumetrics: asset_manager.get_or_load_technique(light.technique_volumetrics),
            technique_compute_lightprobe: asset_manager
                .get_or_load_technique(light.technique_compute_lightprobe),
            debug_label,
            debug_info: format!("{light:X?}"),
            ..Self::new_empty(gctx.clone())?
        })
    }

    pub fn load_shadowing(
        gctx: SharedGpuContext,
        asset_manager: &mut AssetManager,
        light: &SShadowingLight,
        debug_label: String,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            projection_matrix: light.light_to_world,
            technique_shading: asset_manager.get_or_load_technique(light.technique_shading),
            technique_shading_shadowing: Some(
                asset_manager.get_or_load_technique(light.technique_shading_shadowing),
            ),
            technique_volumetrics: asset_manager.get_or_load_technique(light.technique_volumetrics),
            technique_volumetrics_shadowing: Some(
                asset_manager.get_or_load_technique(light.technique_volumetrics_shadowing),
            ),
            technique_compute_lightprobe: asset_manager
                .get_or_load_technique(light.technique_compute_lightprobe),
            technique_compute_lightprobe_shadowing: Some(
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
                .context()
                .OMSetDepthStencilState(Some(&self.depth_state), 0);

            // Layout 1
            //  - float3 v0 : POSITION0, // Format DXGI_FORMAT_R32G32B32_FLOAT size 12
            renderer.gpu.set_input_layout(1);
            renderer.gpu.set_blend_state(8);
            renderer.gpu.context().IASetVertexBuffers(
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

            renderer
                .gpu
                .context()
                .IASetIndexBuffer(Some(&self.ib_cube), DXGI_FORMAT_R16_UINT, 0);

            renderer.gpu.set_input_topology(EPrimitiveType::Triangles);

            renderer
                .gpu
                .context()
                .DrawIndexed(self.cube_index_count, 0, 0);
        }
    }
}

pub enum ShadowPcfSamples {
    Samples13 = 0,
    Samples17 = 1,
    Samples21 = 2,
}

pub fn draw_light_system(renderer: &Renderer, scene: &Scene) {
    profiling::scope!("draw_light_system");
    for (_, (transform, light_renderer, light)) in scene
        .query::<(&Transform, &LightRenderer, &SLight)>()
        .without::<&Hidden>()
        .iter()
    {
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

            let transform_mat = transform_relative.local_to_world();
            let transform_mat_scaled = transform.local_to_world() * light.light_to_world;

            externs.simple_geometry = Some(externs::SimpleGeometry {
                transform: view.world_to_projective * transform_mat_scaled,
            });
            let existing_deflight = externs.deferred_light.as_ref().cloned().unwrap_or_default();
            externs.deferred_light = Some(externs::DeferredLight {
                // TODO(cohae): Used for transforming projective textures (see lamps in Altar of Reflection)
                unk40: Transform::from_translation(view.position.xyz()).local_to_world(),
                unk80: transform_mat,
                unk100: light.unk50,

                ..existing_deflight
            });
        }

        light_renderer.draw(renderer, false);
    }

    for (_, (transform, light_renderer, light, shadowmap)) in scene
        .query::<(
            &Transform,
            &LightRenderer,
            &SShadowingLight,
            Option<&ShadowMapRenderer>,
        )>()
        .without::<&Hidden>()
        .iter()
    {
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

            let transform_mat = transform_relative.local_to_world();
            let transform_mat_scaled = transform.local_to_world() * light.light_to_world;

            externs.simple_geometry = Some(externs::SimpleGeometry {
                transform: view.world_to_projective * transform_mat_scaled,
            });

            let existing_deflight = externs.deferred_light.as_ref().cloned().unwrap_or_default();
            externs.deferred_light = Some(externs::DeferredLight {
                // TODO(cohae): Used for transforming projective textures (see lamps in Altar of Reflection)
                unk40: Transform::from_translation(view.position.xyz()).local_to_world(),
                unk80: transform_mat,
                unk100: light.unk50,
                // unk110: 1.0,
                // unk114: 2000.0,
                // unk118: 1.0,
                // unk11c: 1.0,
                // unk120: 1.0,
                ..existing_deflight
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
                    resolution_width: ShadowMapRenderer::RESOLUTION as f32,
                    resolution_height: ShadowMapRenderer::RESOLUTION as f32,
                    unkc0: shadowmap.camera_to_projective * transform_relative.view_matrix(),
                    unk180: ShadowPcfSamples::Samples21 as u8 as f32,
                    ..existing_shadowmap
                })
            }
        }

        light_renderer.draw(
            renderer,
            shadowmap.is_some() && renderer.render_settings.shadows,
        );
    }
}

pub struct ShadowMapRenderer {
    pub last_update: usize,
    pub stationary_needs_update: bool,

    depth_stationary: ShadowDepthMap,
    depth: ShadowDepthMap,
    viewport: Viewport,

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
    const RESOLUTION: u32 = 1024;
    pub fn new(
        gpu: &GpuContext,
        transform: Transform,
        projection: CameraProjection,
    ) -> anyhow::Result<Self> {
        let depth = ShadowDepthMap::create((Self::RESOLUTION, Self::RESOLUTION), 1, &gpu.device)?;
        let depth_stationary =
            ShadowDepthMap::create((Self::RESOLUTION, Self::RESOLUTION), 1, &gpu.device)?;

        let viewport = Viewport {
            origin: UVec2::ZERO,
            size: UVec2::splat(Self::RESOLUTION),
        };

        let world_to_camera = transform.view_matrix();
        let camera_to_projective = projection.matrix(viewport.aspect_ratio());

        Ok(Self {
            last_update: 0,
            stationary_needs_update: true,
            depth_stationary,
            depth,
            viewport,
            world_to_camera,
            camera_to_projective,
        })
    }

    /// Binds the shadowmap
    pub fn bind_for_generation(
        &mut self,
        transform: &Transform,
        renderer: &Renderer,
        mode: ShadowGenerationMode,
    ) {
        self.world_to_camera = transform.view_matrix();

        unsafe {
            let view = match mode {
                ShadowGenerationMode::StationaryOnly => {
                    renderer.gpu.context().ClearDepthStencilView(
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

            renderer.gpu.context().OMSetRenderTargets(None, view);
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
