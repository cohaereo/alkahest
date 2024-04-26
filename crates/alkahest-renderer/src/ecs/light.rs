use std::sync::Arc;

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
use glam::{Mat4, Vec3, Vec4};
use windows::Win32::Graphics::{
    Direct3D::D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
    Direct3D11::{
        ID3D11Buffer, ID3D11DepthStencilState, ID3D11InputLayout, D3D11_BIND_INDEX_BUFFER,
        D3D11_BIND_VERTEX_BUFFER, D3D11_BUFFER_DESC, D3D11_COMPARISON_ALWAYS,
        D3D11_DEPTH_STENCILOP_DESC, D3D11_DEPTH_STENCIL_DESC, D3D11_DEPTH_WRITE_MASK_ZERO,
        D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_VERTEX_DATA, D3D11_STENCIL_OP_DECR,
        D3D11_STENCIL_OP_INCR, D3D11_STENCIL_OP_KEEP, D3D11_SUBRESOURCE_DATA,
        D3D11_USAGE_IMMUTABLE,
    },
    Dxgi::Common::{DXGI_FORMAT_R16_UINT, DXGI_FORMAT_R32G32B32A32_FLOAT},
};

use crate::{
    camera::Camera,
    ecs::{transform::Transform, Scene},
    gpu::{GpuContext, SharedGpuContext},
    handle::Handle,
    loaders::AssetManager,
    tfx::{externs, externs::ExternStorage, technique::Technique},
};

pub struct LightRenderer {
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
        })
    }

    pub fn load(
        gctx: SharedGpuContext,
        asset_manager: &mut AssetManager,
        light: &SLight,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            technique_shading: asset_manager.get_or_load_technique(light.technique_shading),
            technique_volumetrics: asset_manager.get_or_load_technique(light.technique_volumetrics),
            technique_compute_lightprobe: asset_manager
                .get_or_load_technique(light.technique_compute_lightprobe),
            ..Self::new_empty(gctx.clone())?
        })
    }

    pub fn load_shadowing(
        gctx: SharedGpuContext,
        asset_manager: &mut AssetManager,
        light: &SShadowingLight,
    ) -> anyhow::Result<Self> {
        Ok(Self {
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
            ..Self::new_empty(gctx.clone())?
        })
    }

    fn draw(&self, gctx: &GpuContext, asset_manager: &AssetManager, externs: &mut ExternStorage) {
        unsafe {
            gctx.context()
                .OMSetDepthStencilState(Some(&self.depth_state), 0);

            // Layout 1
            //  - float3 v0 : POSITION0, // Format DXGI_FORMAT_R32G32B32_FLOAT size 12
            gctx.set_input_layout(1);
            gctx.set_blend_state(8);
            gctx.context().IASetVertexBuffers(
                0,
                1,
                Some([Some(self.vb_cube.clone())].as_ptr()),
                Some([12].as_ptr()),
                Some(&0),
            );

            if let Some(tech) = asset_manager.techniques.get(&self.technique_shading) {
                tech.bind(gctx, externs, asset_manager)
                    .expect("Failed to bind technique");
            } else {
                return;
            }

            gctx.context()
                .IASetIndexBuffer(Some(&self.ib_cube), DXGI_FORMAT_R16_UINT, 0);

            gctx.set_input_topology(EPrimitiveType::Triangles);

            gctx.context().DrawIndexed(self.cube_index_count, 0, 0);
        }
    }
}

pub fn draw_light_system(
    gctx: &GpuContext,
    scene: &Scene,
    asset_manager: &AssetManager,
    camera: &Camera,
    externs: &mut ExternStorage,
) {
    profiling::scope!("draw_light_system");
    for (_, (transform, light_renderer, light, bounds)) in scene
        .query::<(&Transform, &LightRenderer, &SLight, Option<&AABB>)>()
        .iter()
    {
        let light_scale = if let Some(bb) = bounds {
            Mat4::from_scale(-(bb.extents() * 3.0))
        } else {
            light.unk60
        };

        // let unk_camera_reversal = camera.world_to_camera; // Mat4::from_translation(camera.position()).inverse();
        let transform_mat = transform.to_mat4();
        externs.simple_geometry = Some(externs::SimpleGeometry {
            transform: camera.world_to_projective * (transform_mat * light_scale),
        });

        externs.deferred_light = Some(externs::DeferredLight {
            // TODO(cohae): Used for transforming projective textures (see lamps in Altar of Reflection)
            unk40: Mat4::from_scale(Vec3::splat(0.15)),
            unk80: transform_mat,
            unkc0: transform.translation.extend(1.0),
            unk110: 1.0,
            unk114: 1.0,
            unk118: 1.0,
            unk11c: 1.0,
            unk120: 1.0,

            ..Default::default()
        });

        light_renderer.draw(gctx, asset_manager, externs);
    }

    for (_, (transform, light_renderer, _light)) in scene
        .query::<(&Transform, &LightRenderer, &SShadowingLight)>()
        .iter()
    {
        let light_scale = Mat4::from_scale(Vec3::splat(-(3000.0 * 4.0)));

        let transform_mat = transform.to_mat4();
        externs.simple_geometry = Some(externs::SimpleGeometry {
            transform: camera.world_to_projective * (transform_mat * light_scale),
        });

        externs.deferred_light = Some(externs::DeferredLight {
            // TODO(cohae): Used for transforming projective textures (see lamps in Altar of Reflection)
            unk40: Mat4::from_scale(Vec3::splat(0.15)),
            unk80: transform_mat,
            unkc0: transform.translation.extend(1.0),
            unk110: 1.0,
            unk114: 1.0,
            unk118: 1.0,
            unk11c: 1.0,
            unk120: 1.0,

            ..Default::default()
        });

        light_renderer.draw(gctx, asset_manager, externs);
    }
}
