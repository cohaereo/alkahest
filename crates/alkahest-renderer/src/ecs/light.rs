use alkahest_data::{
    geometry::EPrimitiveType,
    map::{SLight, SShadowingLight},
    occlusion::AABB,
};
use anyhow::Context;
use destiny_pkg::TagHash;
use genmesh::{
    generators::{IndexedPolygon, SharedVertex},
    Triangulate,
};
use glam::{Mat4, Vec3};
use windows::Win32::Graphics::{
    Direct3D11::{
        ID3D11Buffer, ID3D11DepthStencilState, D3D11_BIND_INDEX_BUFFER, D3D11_BIND_VERTEX_BUFFER,
        D3D11_BUFFER_DESC, D3D11_COMPARISON_ALWAYS, D3D11_DEPTH_STENCILOP_DESC,
        D3D11_DEPTH_STENCIL_DESC, D3D11_DEPTH_WRITE_MASK_ZERO, D3D11_STENCIL_OP_DECR,
        D3D11_STENCIL_OP_INCR, D3D11_STENCIL_OP_KEEP, D3D11_SUBRESOURCE_DATA,
        D3D11_USAGE_IMMUTABLE,
    },
    Dxgi::Common::DXGI_FORMAT_R16_UINT,
};

use crate::{
    camera::Camera,
    ecs::{common::Hidden, transform::Transform, Scene},
    gpu::{GpuContext, SharedGpuContext},
    gpu_event,
    handle::{AssetId, Handle},
    loaders::AssetManager,
    renderer::Renderer,
    tfx::{externs, externs::ExternStorage, technique::Technique},
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

    debug_label: String,
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
        })
    }

    pub fn load(
        gctx: SharedGpuContext,
        asset_manager: &mut AssetManager,
        light: &SLight,
        debug_label: String,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            projection_matrix: light.unk60,
            technique_shading: asset_manager.get_or_load_technique(light.technique_shading),
            technique_volumetrics: asset_manager.get_or_load_technique(light.technique_volumetrics),
            technique_compute_lightprobe: asset_manager
                .get_or_load_technique(light.technique_compute_lightprobe),
            debug_label,
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
            projection_matrix: light.unk60,
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
            ..Self::new_empty(gctx.clone())?
        })
    }

    fn draw(&self, renderer: &Renderer) {
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
            if let Some(tech) = renderer.get_technique_shared(&self.technique_shading) {
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

pub fn draw_light_system(renderer: &Renderer, scene: &Scene) {
    profiling::scope!("draw_light_system");
    for (_, (transform, light_renderer, light)) in scene
        .query::<(&Transform, &LightRenderer, &SLight)>()
        .without::<&Hidden>()
        .iter()
    {
        let transform_mat = transform.local_to_world();
        let transform_mat_scaled = transform.local_to_world() * light.unk60;
        {
            let externs = &mut renderer.data.lock().externs;
            let Some(view) = &externs.view else {
                error!("No view extern bound for light rendering");
                return;
            };

            externs.simple_geometry = Some(externs::SimpleGeometry {
                transform: view.world_to_projective * transform_mat_scaled,
            });

            externs.deferred_light = Some(externs::DeferredLight {
                // TODO(cohae): Used for transforming projective textures (see lamps in Altar of Reflection)
                unk40: Mat4::from_scale(Vec3::splat(0.15)),
                unk80: transform_mat,
                unkc0: transform.translation.extend(1.0),
                unkd0: transform.translation.extend(1.0),
                unke0: transform.translation.extend(1.0),
                unk100: light.unk50,
                unk110: 1.0,
                unk114: 1.0,
                unk118: 1.0,
                unk11c: 1.0,
                unk120: 1.0,

                ..Default::default()
            });
        }

        light_renderer.draw(renderer);
    }

    for (_, (transform, light_renderer, light)) in scene
        .query::<(&Transform, &LightRenderer, &SShadowingLight)>()
        .without::<&Hidden>()
        .iter()
    {
        let transform_mat = transform.local_to_world();
        let transform_mat_scaled = transform.local_to_world() * light.unk60;
        {
            let externs = &mut renderer.data.lock().externs;
            let Some(view) = &externs.view else {
                error!("No view extern bound for light rendering");
                return;
            };

            externs.simple_geometry = Some(externs::SimpleGeometry {
                transform: view.world_to_projective * transform_mat_scaled,
            });

            externs.deferred_light = Some(externs::DeferredLight {
                // TODO(cohae): Used for transforming projective textures (see lamps in Altar of Reflection)
                unk40: Mat4::from_scale(Vec3::splat(0.15)),
                unk80: transform_mat,
                unkc0: transform.translation.extend(1.0),
                unkd0: transform.translation.extend(1.0),
                unke0: transform.translation.extend(1.0),
                unk100: light.unk50,
                unk110: 1.0,
                unk114: 1.0,
                unk118: 1.0,
                unk11c: 1.0,
                unk120: 1.0,

                ..Default::default()
            });
        }

        light_renderer.draw(renderer);
    }
}
