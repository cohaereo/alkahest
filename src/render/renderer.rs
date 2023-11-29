use std::{sync::Arc, time::Instant};

use crate::ecs::transform::Transform;
use crate::map::{MapDataList, SLight, SShadowingLight};
use crate::overlays::camera_settings::CurrentCubemap;
use crate::types::AABB;
use crate::util::RwLock;
use glam::{Mat4, Quat, Vec3, Vec4};
use windows::Win32::Graphics::Direct3D::D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT;
use winit::window::Window;

use crate::overlays::render_settings::{CompositorOptions, RenderSettings};
use crate::render::drawcall::ShaderStages;
use crate::render::scopes::ScopeUnk3;
use crate::render::shader;
use crate::{camera::FpsCamera, resources::Resources};

use super::bytecode::externs::TfxShaderStage;
use super::data::RenderDataManager;
use super::debug::{DebugShapeRenderer, DebugShapes};
use super::drawcall::{GeometryType, Transparency};
use super::gbuffer::ShadowDepthMap;
use super::light::LightRenderer;
use super::overrides::{EnabledShaderOverrides, ScopeOverrides, ShaderOverrides};
use super::scopes::{ScopeUnk2, ScopeUnk8};
use super::{
    drawcall::{DrawCall, ShadingMode, SortValue3d},
    scopes::{ScopeFrame, ScopeView},
    ConstantBuffer, DeviceContextSwapchain, GBuffer,
};

#[derive(PartialEq, Eq)]
enum RendererState {
    /// The renderer is waiting to record a new frame
    Awaiting,
    /// The renderer is recording drawcalls for a frame
    Recording,
}

pub type RendererShared = Arc<RwLock<Renderer>>;
pub struct Renderer {
    draw_queue: RwLock<Vec<(SortValue3d, DrawCall)>>,
    state: RwLock<RendererState>,

    pub gbuffer: GBuffer,
    window_size: (u32, u32),
    pub dcs: Arc<DeviceContextSwapchain>,

    scope_view_backup: RwLock<ScopeView>,
    scope_view: ConstantBuffer<ScopeView>,
    scope_view_pixel: ConstantBuffer<ScopeView>,

    scope_view_csm: ConstantBuffer<ScopeView>,
    scope_frame: ConstantBuffer<ScopeFrame>,
    scope_unk2: ConstantBuffer<ScopeUnk2>,
    scope_unk3: ConstantBuffer<ScopeUnk3>,
    scope_unk8: ConstantBuffer<ScopeUnk8>,
    scope_alk_composite: ConstantBuffer<CompositorOptions>,
    scope_alk_cascade_transforms: ConstantBuffer<[Mat4; Self::CAMERA_CASCADE_LEVEL_COUNT]>,

    pub start_time: Instant,
    pub last_frame: RwLock<Instant>,
    pub delta_time: RwLock<f32>,

    pub render_data: RenderDataManager,

    blend_state_none: ID3D11BlendState,
    blend_state_blend: ID3D11BlendState,
    pub blend_state_additive: ID3D11BlendState,
    blend_state_decals: ID3D11BlendState,

    pub rasterizer_state: ID3D11RasterizerState,
    pub rasterizer_state_nocull: ID3D11RasterizerState,

    composite_vs: ID3D11VertexShader,
    composite_ps: ID3D11PixelShader,

    final_vs: ID3D11VertexShader,
    final_ps: ID3D11PixelShader,

    null_ps: ID3D11PixelShader,

    debug_shape_renderer: DebugShapeRenderer,

    shader_overrides: ShaderOverrides,

    light_cascade_transforms: RwLock<[Mat4; Self::CAMERA_CASCADE_LEVEL_COUNT]>,
    shadow_rs: ID3D11RasterizerState,

    last_material: RwLock<u32>,

    // TODO(cohae): find a better way to get the light transform into the bytecode interpreter
    pub light_transform: RwLock<Transform>,
    pub light_mat: RwLock<Mat4>,
    pub light_mul: RwLock<f32>,
    // TODO(cohae): AAAAAAAAAAAA
    pub camera_viewproj: RwLock<Mat4>,
    pub camera_svp_inv: RwLock<Mat4>,
    light_renderer: LightRenderer,
}

impl Renderer {
    pub fn create(window: &Window, dcs: Arc<DeviceContextSwapchain>) -> anyhow::Result<Self> {
        let blend_state_none = unsafe {
            dcs.device.CreateBlendState(&D3D11_BLEND_DESC {
                RenderTarget: [D3D11_RENDER_TARGET_BLEND_DESC {
                    BlendEnable: false.into(),
                    SrcBlend: D3D11_BLEND_ONE,
                    DestBlend: D3D11_BLEND_ZERO,
                    BlendOp: D3D11_BLEND_OP_ADD,
                    SrcBlendAlpha: D3D11_BLEND_ONE,
                    DestBlendAlpha: D3D11_BLEND_ZERO,
                    BlendOpAlpha: D3D11_BLEND_OP_ADD,
                    RenderTargetWriteMask: (D3D11_COLOR_WRITE_ENABLE_RED.0
                        | D3D11_COLOR_WRITE_ENABLE_BLUE.0
                        | D3D11_COLOR_WRITE_ENABLE_GREEN.0)
                        as u8,
                }; 8],
                ..Default::default()
            })?
        };

        let blend_state_blend = unsafe {
            dcs.device.CreateBlendState(&D3D11_BLEND_DESC {
                RenderTarget: [D3D11_RENDER_TARGET_BLEND_DESC {
                    BlendEnable: true.into(),
                    SrcBlend: D3D11_BLEND_SRC_ALPHA,
                    DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
                    BlendOp: D3D11_BLEND_OP_ADD,
                    SrcBlendAlpha: D3D11_BLEND_ONE,
                    DestBlendAlpha: D3D11_BLEND_ZERO,
                    BlendOpAlpha: D3D11_BLEND_OP_ADD,
                    RenderTargetWriteMask: (D3D11_COLOR_WRITE_ENABLE_RED.0
                        | D3D11_COLOR_WRITE_ENABLE_BLUE.0
                        | D3D11_COLOR_WRITE_ENABLE_GREEN.0)
                        as u8,
                }; 8],
                ..Default::default()
            })?
        };

        let blend_state_additive = unsafe {
            dcs.device.CreateBlendState(&D3D11_BLEND_DESC {
                RenderTarget: [D3D11_RENDER_TARGET_BLEND_DESC {
                    BlendEnable: true.into(),
                    SrcBlend: D3D11_BLEND_ONE,
                    DestBlend: D3D11_BLEND_ONE,
                    BlendOp: D3D11_BLEND_OP_ADD,
                    SrcBlendAlpha: D3D11_BLEND_ONE,
                    DestBlendAlpha: D3D11_BLEND_ZERO,
                    BlendOpAlpha: D3D11_BLEND_OP_ADD,
                    RenderTargetWriteMask: (D3D11_COLOR_WRITE_ENABLE_RED.0
                        | D3D11_COLOR_WRITE_ENABLE_BLUE.0
                        | D3D11_COLOR_WRITE_ENABLE_GREEN.0)
                        as u8,
                }; 8],
                ..Default::default()
            })?
        };

        let blend_state_decals = unsafe {
            dcs.device.CreateBlendState(&D3D11_BLEND_DESC {
                RenderTarget: [D3D11_RENDER_TARGET_BLEND_DESC {
                    BlendEnable: true.into(),
                    SrcBlend: D3D11_BLEND_SRC_COLOR,
                    DestBlend: D3D11_BLEND_DEST_COLOR,
                    BlendOp: D3D11_BLEND_OP_MAX,
                    SrcBlendAlpha: D3D11_BLEND_ONE,
                    DestBlendAlpha: D3D11_BLEND_ZERO,
                    BlendOpAlpha: D3D11_BLEND_OP_MAX,
                    RenderTargetWriteMask: (D3D11_COLOR_WRITE_ENABLE_RED.0
                        | D3D11_COLOR_WRITE_ENABLE_BLUE.0
                        | D3D11_COLOR_WRITE_ENABLE_GREEN.0)
                        as u8,
                }; 8],
                IndependentBlendEnable: false.into(),
                ..Default::default()
            })?
        };

        let rasterizer_state = unsafe {
            dcs.device.CreateRasterizerState(&D3D11_RASTERIZER_DESC {
                FillMode: D3D11_FILL_SOLID,
                CullMode: D3D11_CULL_BACK,
                FrontCounterClockwise: true.into(),
                DepthBias: 0,
                DepthBiasClamp: 0.0,
                SlopeScaledDepthBias: 0.0,
                DepthClipEnable: true.into(),
                ScissorEnable: Default::default(),
                MultisampleEnable: Default::default(),
                AntialiasedLineEnable: Default::default(),
            })?
        };

        let rasterizer_state_nocull = unsafe {
            dcs.device.CreateRasterizerState(&D3D11_RASTERIZER_DESC {
                FillMode: D3D11_FILL_SOLID,
                CullMode: D3D11_CULL_NONE,
                FrontCounterClockwise: true.into(),
                DepthBias: 0,
                DepthBiasClamp: 0.0,
                SlopeScaledDepthBias: 0.0,
                DepthClipEnable: true.into(),
                ScissorEnable: Default::default(),
                MultisampleEnable: Default::default(),
                AntialiasedLineEnable: Default::default(),
            })?
        };

        let shadow_rs = unsafe {
            dcs.device.CreateRasterizerState(&D3D11_RASTERIZER_DESC {
                FillMode: D3D11_FILL_SOLID,
                CullMode: D3D11_CULL_BACK,
                FrontCounterClockwise: true.into(),
                DepthBias: 0,
                DepthBiasClamp: 0.0,
                SlopeScaledDepthBias: 0.0,
                DepthClipEnable: false.into(),
                ScissorEnable: Default::default(),
                MultisampleEnable: Default::default(),
                AntialiasedLineEnable: Default::default(),
            })?
        };

        let vshader_composite_blob = shader::compile_hlsl(
            include_str!("../../assets/shaders/composite.hlsl"),
            "VShader",
            "vs_5_0",
        )
        .unwrap();
        let pshader_composite_blob = shader::compile_hlsl(
            include_str!("../../assets/shaders/composite.hlsl"),
            "PShader",
            "ps_5_0",
        )
        .unwrap();

        let (vshader_composite, _) = shader::load_vshader(&dcs, &vshader_composite_blob)?;
        let (pshader_composite, _) = shader::load_pshader(&dcs, &pshader_composite_blob)?;

        let vshader_final_blob = shader::compile_hlsl(
            include_str!("../../assets/shaders/final.hlsl"),
            "VShader",
            "vs_5_0",
        )
        .unwrap();
        let pshader_final_blob = shader::compile_hlsl(
            include_str!("../../assets/shaders/final.hlsl"),
            "PShader",
            "ps_5_0",
        )
        .unwrap();

        let (vshader_final, _) = shader::load_vshader(&dcs, &vshader_final_blob)?;
        let (pshader_final, _) = shader::load_pshader(&dcs, &pshader_final_blob)?;

        let pshader_null_blob = shader::compile_hlsl(
            include_str!("../../assets/shaders/null.hlsl"),
            "main",
            "ps_5_0",
        )
        .unwrap();
        let (pshader_null, _) = shader::load_pshader(&dcs, &pshader_null_blob)?;

        Ok(Renderer {
            light_cascade_transforms: RwLock::new(
                [Mat4::IDENTITY; Self::CAMERA_CASCADE_LEVEL_COUNT],
            ),
            shader_overrides: ShaderOverrides::load(&dcs)?,
            debug_shape_renderer: DebugShapeRenderer::new(dcs.clone())?,
            draw_queue: RwLock::new(Vec::with_capacity(8192)),
            state: RwLock::new(RendererState::Awaiting),
            gbuffer: GBuffer::create(
                (window.inner_size().width, window.inner_size().height),
                dcs.clone(),
            )?,
            window_size: (window.inner_size().width, window.inner_size().height),
            scope_frame: ConstantBuffer::create(dcs.clone(), None)?,
            scope_view_backup: RwLock::new(ScopeView::default()),
            scope_view: ConstantBuffer::create(dcs.clone(), None)?,
            scope_view_pixel: ConstantBuffer::create(dcs.clone(), None)?,
            scope_view_csm: ConstantBuffer::create(dcs.clone(), None)?,
            scope_unk2: ConstantBuffer::create(dcs.clone(), None)?,
            scope_unk3: ConstantBuffer::create(dcs.clone(), None)?,
            scope_unk8: ConstantBuffer::create(dcs.clone(), None)?,
            scope_alk_composite: ConstantBuffer::create(dcs.clone(), None)?,
            scope_alk_cascade_transforms: ConstantBuffer::create(dcs.clone(), None)?,
            render_data: RenderDataManager::new(dcs.clone()),
            light_renderer: LightRenderer::new(dcs.clone())?,
            dcs,
            start_time: Instant::now(),
            last_frame: RwLock::new(Instant::now()),
            delta_time: RwLock::new(0.016),
            blend_state_none,
            blend_state_blend,
            blend_state_additive,
            blend_state_decals,
            rasterizer_state,
            rasterizer_state_nocull,
            shadow_rs,
            composite_vs: vshader_composite,
            composite_ps: pshader_composite,
            final_vs: vshader_final,
            final_ps: pshader_final,
            null_ps: pshader_null,
            last_material: RwLock::new(u32::MAX),
            light_mat: RwLock::new(Mat4::IDENTITY),
            light_transform: RwLock::new(Transform {
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            }),
            camera_viewproj: RwLock::new(Mat4::IDENTITY),
            camera_svp_inv: RwLock::new(Mat4::IDENTITY),
            light_mul: RwLock::new(1.0),
        })
    }

    pub fn begin_frame(&self) {
        if *self.state.read() == RendererState::Recording {
            panic!("Called begin(), but a frame is already being recorded! Did you call submit()?")
        }

        *self.delta_time.write() = self.last_frame.read().elapsed().as_secs_f32();
        *self.last_frame.write() = Instant::now();

        self.draw_queue.write().clear();
        *self.state.write() = RendererState::Recording;
    }

    // TODO(cohae): `begin` should probably return a CommandEncoder that we can record stuff in
    pub fn push_drawcall(&self, ordering: SortValue3d, drawcall: DrawCall) {
        self.draw_queue.write().push((ordering, drawcall))
    }

    /// Submits recorded drawcalls
    pub fn submit_frame(&self, resources: &Resources) {
        if *self.state.read() != RendererState::Recording {
            panic!("Called submit(), but the renderer is not recording! Did you call begin()?")
        }

        self.draw_queue
            .write()
            .sort_unstable_by(|(o1, _), (o2, _)| o1.cmp(o2));

        self.update_buffers(resources)
            .expect("Renderer::update_buffers");

        let render_settings = resources.get::<RenderSettings>().unwrap();

        self.scope_unk2.bind(2, TfxShaderStage::Vertex);
        self.scope_unk2.bind(2, TfxShaderStage::Pixel);
        self.scope_unk3.bind(3, TfxShaderStage::Vertex);
        self.scope_unk3.bind(3, TfxShaderStage::Pixel);
        self.scope_unk8.bind(8, TfxShaderStage::Vertex);
        self.scope_unk8.bind(8, TfxShaderStage::Pixel);
        self.scope_frame.bind(13, TfxShaderStage::Vertex);
        self.scope_frame.bind(13, TfxShaderStage::Pixel);

        if render_settings.draw_lights {
            self.render_cascade_depthmaps(resources);
        }

        self.scope_view.bind(12, TfxShaderStage::Vertex);
        self.scope_view_pixel.bind(12, TfxShaderStage::Pixel);

        unsafe {
            self.dcs.context().RSSetState(&self.shadow_rs);
            self.dcs.context().OMSetRenderTargets(
                Some(&[
                    Some(self.gbuffer.rt0.render_target.clone()),
                    Some(self.gbuffer.rt1.render_target.clone()),
                    Some(self.gbuffer.rt2.render_target.clone()),
                    Some(self.gbuffer.rt3.render_target.clone()),
                ]),
                &self.gbuffer.depth.view,
            );
            self.dcs
                .context()
                .OMSetDepthStencilState(&self.gbuffer.depth.state, 0);
            self.dcs.context().OMSetBlendState(
                &self.blend_state_none,
                Some(&[1f32, 1., 1., 1.] as _),
                0xffffffff,
            );

            self.dcs.context().RSSetViewports(Some(&[D3D11_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: self.window_size.0 as f32,
                Height: self.window_size.1 as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }]));
        }

        let shader_overrides = resources.get::<EnabledShaderOverrides>().unwrap();

        //region Deferred
        let draw_queue = self.draw_queue.read();
        for i in 0..draw_queue.len() {
            if draw_queue[i].0.shading_mode() != ShadingMode::Deferred
                || draw_queue[i].0.geometry_type() == GeometryType::StaticDecal
            {
                continue;
            }

            let (s, d) = draw_queue[i].clone();
            self.draw(
                s,
                &d,
                &shader_overrides,
                DrawMode::Normal,
                render_settings.evaluate_bytecode,
            );
        }
        //endregion

        //region Deferred (decals)
        self.gbuffer.rt1.copy_to(&self.gbuffer.rt1_clone);
        let draw_queue = self.draw_queue.read();
        for i in 0..draw_queue.len() {
            if draw_queue[i].0.shading_mode() != ShadingMode::Deferred
                || draw_queue[i].0.geometry_type() != GeometryType::StaticDecal
            {
                continue;
            }

            unsafe {
                // Necessary for propery decal mesh blending
                self.dcs
                    .context()
                    .PSSetShaderResources(2, Some(&[Some(self.gbuffer.rt1_clone.view.clone())]));
            }

            let (s, d) = draw_queue[i].clone();
            self.draw(
                s,
                &d,
                &shader_overrides,
                DrawMode::Normal,
                render_settings.evaluate_bytecode,
            );
        }
        //endregion

        self.gbuffer.depth.copy_depth(self.dcs.context());

        self.run_deferred_shading(
            resources,
            render_settings.draw_lights,
            render_settings.use_global_deferred_shading,
            render_settings.compositor_mode,
        );

        unsafe {
            self.dcs
                .context()
                .OMSetDepthStencilState(&self.gbuffer.depth.state, 0);

            self.dcs.context().OMSetRenderTargets(
                Some(&[Some(
                    self.gbuffer.staging.render_target.clone(), // self.dcs.swapchain_target.read().as_ref().unwrap().clone(),
                )]),
                &self.gbuffer.depth.view,
            );
        }

        self.gbuffer.staging.copy_to(&self.gbuffer.staging_clone);
        //region Forward
        let mut transparency_mode = Transparency::None;
        for i in 0..draw_queue.len() {
            if draw_queue[i].0.shading_mode() != ShadingMode::Forward {
                continue;
            }

            self.render_data
                .data()
                .blend_texture
                .bind(&self.dcs, 0, ShaderStages::PIXEL);
            for i in 1..8 {
                self.render_data.data().debug_textures[i].bind(
                    &self.dcs,
                    i as u32,
                    ShaderStages::all(),
                );
            }

            self.render_data
                .data()
                .blend_texture15
                .bind(&self.dcs, 15, ShaderStages::all());

            for (i, slot) in (16..24).filter(|&v| v != 14).enumerate() {
                self.render_data.data().debug_textures[i].bind(
                    &self.dcs,
                    slot,
                    ShaderStages::all(),
                );
            }

            unsafe {
                self.dcs.context().PSSetShaderResources(
                    10,
                    Some(&[Some(self.gbuffer.depth.texture_copy_view.clone())]),
                );
                self.dcs.context().PSSetShaderResources(
                    11,
                    Some(&[Some(self.gbuffer.staging_clone.view.clone())]),
                );
                self.dcs.context().PSSetShaderResources(
                    13,
                    Some(&[Some(self.gbuffer.staging_clone.view.clone())]),
                );
                self.dcs.context().PSSetShaderResources(
                    20,
                    Some(&[Some(self.gbuffer.staging_clone.view.clone())]),
                );
                self.dcs.context().PSSetShaderResources(
                    23,
                    Some(&[Some(self.gbuffer.staging_clone.view.clone())]),
                );

                let cubemap_texture = resources.get::<CurrentCubemap>().unwrap().1.and_then(|t| {
                    self.render_data
                        .data()
                        .textures
                        .get(&t.key())
                        .map(|t| t.view.clone())
                });

                self.dcs
                    .context()
                    .PSSetShaderResources(24, Some(&[cubemap_texture]));
            }
            // self.render_data
            //     .data()
            //     .white
            //     .bind(&self.dcs, 20, ShaderStages::all());
            // self.render_data
            //     .data()
            //     .rainbow_texture
            //     .bind(&self.dcs, 21, ShaderStages::all());
            self.render_data
                .data()
                .blend_texture
                .bind(&self.dcs, 21, ShaderStages::all());

            let (s, d) = draw_queue[i].clone();
            if s.transparency() != transparency_mode {
                if render_settings.alpha_blending {
                    // Swap to read-only depth state once we start rendering translucent geometry
                    if s.transparency() != Transparency::None
                        && s.transparency() != Transparency::Cutout
                    {
                        unsafe {
                            self.dcs
                                .context()
                                .OMSetDepthStencilState(&self.gbuffer.depth.state_readonly, 0);
                        }
                    }

                    unsafe {
                        match render_settings.blend_override {
                            1 => self.dcs.context().OMSetBlendState(
                                &self.blend_state_blend,
                                Some(&[1f32, 1., 1., 1.] as _),
                                0xffffffff,
                            ),
                            2 => self.dcs.context().OMSetBlendState(
                                &self.blend_state_additive,
                                Some(&[1f32, 1., 1., 1.] as _),
                                0xffffffff,
                            ),
                            3 => continue,
                            _ => match s.transparency() {
                                Transparency::Blend => self.dcs.context().OMSetBlendState(
                                    &self.blend_state_blend,
                                    Some(&[1f32, 1., 1., 1.] as _),
                                    0xffffffff,
                                ),

                                Transparency::Additive => self.dcs.context().OMSetBlendState(
                                    &self.blend_state_additive,
                                    Some(&[1f32, 1., 1., 1.] as _),
                                    0xffffffff,
                                ),
                                _ => {}
                            },
                        }
                    }
                }

                transparency_mode = s.transparency();
            }

            self.draw(
                s,
                &d,
                &shader_overrides,
                DrawMode::Normal,
                render_settings.evaluate_bytecode,
            );
        }
        //endregion

        self.run_final();

        self.scope_alk_composite.bind(0, TfxShaderStage::Vertex);
        self.scope_alk_composite.bind(0, TfxShaderStage::Pixel);
        if let Some(mut shapes) = resources.get_mut::<DebugShapes>() {
            unsafe {
                self.dcs.context().OMSetRenderTargets(
                    Some(&[Some(
                        self.dcs.swapchain_target.read().as_ref().unwrap().clone(),
                    )]),
                    &self.gbuffer.depth.view,
                );

                self.dcs
                    .context()
                    .OMSetDepthStencilState(&self.gbuffer.depth.state_readonly, 0);

                self.dcs.context().OMSetBlendState(
                    &self.blend_state_blend,
                    Some(&[1f32, 1., 1., 1.] as _),
                    0xffffffff,
                );
                self.dcs.context().RSSetState(&self.rasterizer_state);
            }
            self.debug_shape_renderer.draw_all(&mut shapes);
        }

        *self.state.write() = RendererState::Awaiting;
    }

    fn draw(
        &self,
        sort: SortValue3d,
        drawcall: &DrawCall,
        shader_overrides: &EnabledShaderOverrides,
        mode: DrawMode,
        evaluate_tfx_bytecode: bool,
    ) {
        if mode == DrawMode::DepthPrepass && !sort.transparency().should_write_depth() {
            return;
        }

        // // Workaround for some weird textures that aren't bound by the material
        // self.white.bind(&self.dcs, 0, ShaderStages::all());
        // self.white.bind(&self.dcs, 1, ShaderStages::all());
        // self.white.bind(&self.dcs, 2, ShaderStages::all());

        let bind_stages = match mode {
            DrawMode::Normal => ShaderStages::VERTEX | ShaderStages::PIXEL,
            // Don't bother binding anything for the pixel stage
            DrawMode::DepthPrepass => ShaderStages::VERTEX,
        };

        let render_data = self.render_data.data();

        if let Some(dyemap) = drawcall.dyemap {
            unsafe {
                self.dcs.context().PSSetShaderResources(
                    14,
                    Some(&[render_data
                        .textures
                        .get(&(dyemap.0 as u64))
                        .map(|t| t.view.clone())]),
                );
            }
        }

        if let Some(mat) = render_data.materials.get(&sort.material().into()) {
            if mat.unk8 != 1 || (mat.unk20 & 0x8000) != 0 {
                return;
            }

            unsafe {
                if mode != DrawMode::DepthPrepass {
                    if mat.unkc != 0 {
                        self.dcs.context().RSSetState(&self.rasterizer_state_nocull);
                    } else {
                        self.dcs.context().RSSetState(&self.rasterizer_state);
                    }
                }
            }

            // TODO(cohae): How can we handle these errors?
            if mat.bind(&self.dcs, &render_data, bind_stages).is_err() {
                // return;
            }
        } else {
            // return;
        }

        if let Some(variant_material) = drawcall.variant_material {
            if let Some(mat) = render_data.materials.get(&variant_material) {
                if mat.unk8 != 1 || (mat.unk20 & 0x8000) != 0 {
                    return;
                }

                if mat.bind(&self.dcs, &render_data, bind_stages).is_err() {
                    // return;
                }
            } else {
                // return;
            }
        }

        match sort.geometry_type() {
            GeometryType::Static => {}
            GeometryType::StaticDecal => {
                if sort.shading_mode() == ShadingMode::Deferred {
                    unsafe {
                        self.dcs
                            .context()
                            .OMSetBlendState(&self.blend_state_decals, None, u32::MAX)
                    }
                }
            }
            GeometryType::Terrain => unsafe {
                if shader_overrides.terrain_ps {
                    self.dcs
                        .context()
                        .PSSetShader(&self.shader_overrides.terrain_ps, None);
                    self.dcs.context().PSSetSamplers(
                        0,
                        Some(&[Some(self.shader_overrides.terrain_debug_sampler.clone())]),
                    );
                }
            },
            GeometryType::Entity => unsafe {
                if shader_overrides.entity_vs {
                    self.dcs
                        .context()
                        .VSSetShader(&self.shader_overrides.entity_vs, None);
                }
                if shader_overrides.entity_ps {
                    self.dcs.context().PSSetShader(
                        if sort.shading_mode() == ShadingMode::Deferred {
                            &self.shader_overrides.entity_ps_deferred
                        } else {
                            &self.shader_overrides.entity_ps_forward
                        },
                        None,
                    );
                }
            },
        }

        if mode == DrawMode::DepthPrepass {
            unsafe {
                self.dcs.context().PSSetShader(&self.null_ps, None);
            }
        }

        if let Some(color_buffer) = drawcall.color_buffer {
            if let Some((_buffer, _, Some(srv))) = render_data.vertex_buffers.get(&color_buffer) {
                unsafe {
                    self.dcs
                        .context()
                        .VSSetShaderResources(0, Some(&[Some(srv.clone())]))
                }
            }
        }

        if evaluate_tfx_bytecode {
            let mut last_material = self.last_material.write();
            if *last_material != sort.material() {
                if let Some(mat) = render_data.materials.get(&sort.material().into()) {
                    mat.evaluate_bytecode(self, &render_data)
                }

                *last_material = sort.material()
            }

            if let Some(variant_material) = drawcall.variant_material {
                if let Some(mat) = render_data.materials.get(&variant_material) {
                    mat.evaluate_bytecode(self, &render_data)
                }
            }
        }

        unsafe {
            for b in &drawcall.buffer_bindings {
                self.dcs
                    .context()
                    .VSSetConstantBuffers(b.slot, Some(&[Some(b.buffer.clone())]));

                if bind_stages.contains(ShaderStages::PIXEL) {
                    self.dcs
                        .context()
                        .PSSetConstantBuffers(b.slot, Some(&[Some(b.buffer.clone())]));
                }
            }

            if let Some(input_layout) = render_data.input_layouts.get(&drawcall.input_layout_hash) {
                self.dcs.context().IASetInputLayout(input_layout);
            } else {
                panic!(
                    "Couldn't find input layout 0x{:x}",
                    drawcall.input_layout_hash
                );
            }

            for (buffer_index, vb) in drawcall.vertex_buffers.iter().enumerate() {
                if !vb.is_some() {
                    continue;
                }

                if let Some((buffer, stride, _)) = render_data.vertex_buffers.get(vb) {
                    self.dcs.context().IASetVertexBuffers(
                        buffer_index as _,
                        1,
                        Some([Some(buffer.clone())].as_ptr()),
                        Some([*stride].as_ptr()),
                        Some(&0),
                    );
                } else {
                    // error!("Couldn't bind vertex buffer {}", vb);
                    return;
                }
            }

            if let Some((index_buffer, index_buffer_format)) =
                render_data.index_buffers.get(&drawcall.index_buffer)
            {
                self.dcs.context().IASetIndexBuffer(
                    Some(index_buffer),
                    DXGI_FORMAT(*index_buffer_format as _),
                    0,
                );
            } else {
                // error!("Couldn't bind index buffer {}", drawcall.index_buffer);
                return;
            }

            self.dcs
                .context()
                .IASetPrimitiveTopology(drawcall.primitive_type);

            if drawcall.instance_start.is_some() || drawcall.instance_count.is_some() {
                self.dcs.context().DrawIndexedInstanced(
                    drawcall.index_count,
                    drawcall.instance_count.unwrap_or(1) as _,
                    drawcall.index_start,
                    0,
                    drawcall.instance_start.unwrap_or(0),
                );
            } else {
                self.dcs
                    .context()
                    .DrawIndexed(drawcall.index_count, drawcall.index_start, 0);
            }
        }

        if let Some(mat) = render_data.materials.get(&sort.material().into()) {
            mat.unbind_textures(&self.dcs)
        }

        if let Some(variant_material) = drawcall.variant_material {
            if let Some(mat) = render_data.materials.get(&variant_material) {
                mat.unbind_textures(&self.dcs)
            }
        }
    }

    /// Swaps to primary swapchain render target, binds gbuffers and runs the shading passes
    fn run_deferred_shading(
        &self,
        resources: &Resources,
        draw_lights: bool,
        use_global_deferred_shading: bool,
        compositor_mode: usize,
    ) {
        let maps = resources.get::<MapDataList>().unwrap();

        unsafe {
            let ambient_light = resources.get::<RenderSettings>().unwrap().ambient_light;
            self.dcs.context().ClearRenderTargetView(
                &self.gbuffer.light_diffuse.render_target,
                [ambient_light.x, ambient_light.y, ambient_light.z, 0.0].as_ptr() as _,
            );
            self.dcs.context().ClearRenderTargetView(
                &self.gbuffer.light_specular.render_target,
                [0.0, 0.0, 0.0, 0.0].as_ptr() as _,
            );
        }

        if draw_lights {
            unsafe {
                self.dcs.context().RSSetState(&self.rasterizer_state);
                self.dcs.context().OMSetRenderTargets(
                    Some(&[
                        Some(self.gbuffer.light_diffuse.render_target.clone()),
                        Some(self.gbuffer.light_specular.render_target.clone()),
                    ]),
                    None,
                );
            }

            if let Some((_, _, map)) = maps.current_map() {
                for (_, (transform, light, bounds)) in map
                    .scene
                    .query::<(&Transform, &SLight, Option<&AABB>)>()
                    .iter()
                {
                    if let Some(bb) = bounds {
                        *self.light_mat.write() = Mat4::from_scale(-(bb.extents() * 4.0));
                    } else {
                        *self.light_mat.write() = light.unk60.into();
                    }

                    *self.light_transform.write() = *transform;
                    self.light_renderer.draw_normal(self, light);
                }

                for (_, (transform, light)) in
                    map.scene.query::<(&Transform, &SShadowingLight)>().iter()
                {
                    // *self.light_mat.write() = light.unk64.into();
                    *self.light_mat.write() = Mat4::from_scale(Vec3::splat(-(3000.0 * 2.0)));
                    *self.light_transform.write() = *transform;
                    self.light_renderer.draw_shadowing(self, light);
                }
            }
        }

        unsafe {
            self.dcs.context().OMSetBlendState(
                &self.blend_state_none,
                Some(&[1f32, 1., 1., 1.] as _),
                0xffffffff,
            );

            self.dcs.context().OMSetRenderTargets(
                Some(&[Some(self.gbuffer.staging.render_target.clone())]),
                None,
            );
            self.dcs.context().ClearRenderTargetView(
                &self.gbuffer.staging.render_target,
                [0.0, 0.0, 0.0, 0.0].as_ptr() as _,
            );
        }

        if use_global_deferred_shading {
            let render_data = self.render_data.data();

            if let Some(mat) = &render_data.technique_deferred_shading_no_atm {
                mat.evaluate_bytecode(self, &render_data);
                if let Err(e) = mat.bind(&self.dcs, &render_data, ShaderStages::SHADING) {
                    error!("Failed to run deferred_shading_no_atm: {e}");
                    return;
                }

                unsafe {
                    self.dcs
                        .context()
                        .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
                    self.dcs.context().Draw(6, 0);
                }
            }
        } else {
            unsafe {
                self.dcs.context().PSSetShaderResources(
                    0,
                    Some(&[
                        Some(self.gbuffer.rt0.view.clone()),
                        Some(self.gbuffer.rt1.view.clone()),
                        Some(self.gbuffer.rt2.view.clone()),
                        Some(self.gbuffer.rt3.view.clone()),
                        Some(self.gbuffer.depth.texture_view.clone()),
                    ]),
                );
                self.dcs.context().PSSetShaderResources(
                    12,
                    Some(&[
                        Some(self.gbuffer.light_diffuse.view.clone()),
                        Some(self.gbuffer.light_specular.view.clone()),
                    ]),
                );
                self.dcs.context().PSSetShaderResources(
                    10,
                    Some(&[Some(
                        resources
                            .get::<ShadowMapsResource>()
                            .unwrap()
                            .cascade_depth_buffers
                            .texture_view
                            .clone(),
                    )]),
                );

                self.render_data
                    .data()
                    .matcap
                    .bind(&self.dcs, 8, ShaderStages::PIXEL);

                let cubemap_texture = resources.get::<CurrentCubemap>().unwrap().1.and_then(|t| {
                    self.render_data
                        .data()
                        .textures
                        .get(&t.key())
                        .map(|t| t.view.clone())
                });

                self.dcs
                    .context()
                    .PSSetShaderResources(9, Some(&[cubemap_texture]));

                {
                    let camera = resources.get::<FpsCamera>().unwrap();
                    let render_settings = resources.get::<RenderSettings>().unwrap();
                    *self.light_mul.write() = render_settings.light_mul;

                    let view = camera.calculate_matrix();
                    let proj_view = camera.projection_matrix * view;
                    let view = Mat4::from_translation(camera.position);
                    let compositor_options = CompositorOptions {
                        viewport_proj_view_matrix_inv: *self.camera_svp_inv.read(),
                        proj_view_matrix_inv: proj_view.inverse(),
                        proj_view_matrix: proj_view,
                        proj_matrix: camera.projection_matrix,
                        view_matrix: view,
                        camera_pos: camera.position.extend(1.0),
                        camera_dir: camera.front.extend(1.0),
                        time: self.start_time.elapsed().as_secs_f32(),
                        mode: compositor_mode as u32,
                        draw_lights: draw_lights.into(),
                        global_light_dir: render_settings.light_dir.extend(1.0),
                        global_light_color: render_settings.light_color,
                        specular_scale: if render_settings.use_specular_map {
                            1.0
                        } else {
                            0.0
                        },
                        fxaa_enabled: if render_settings.fxaa { 1 } else { 0 },
                    };
                    self.scope_alk_composite.write(&compositor_options).unwrap();
                    self.scope_alk_composite.bind(0, TfxShaderStage::Vertex);
                    self.scope_alk_composite.bind(0, TfxShaderStage::Pixel);
                }
                self.scope_alk_cascade_transforms
                    .bind(3, TfxShaderStage::Pixel);

                self.dcs.context().RSSetViewports(Some(&[D3D11_VIEWPORT {
                    TopLeftX: 0.0,
                    TopLeftY: 0.0,
                    Width: self.window_size.0 as f32,
                    Height: self.window_size.1 as f32,
                    MinDepth: 0.0,
                    MaxDepth: 1.0,
                }]));

                self.dcs.context().VSSetShader(&self.composite_vs, None);
                self.dcs.context().PSSetShader(&self.composite_ps, None);
                self.dcs
                    .context()
                    .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
                self.dcs.context().Draw(4, 0);

                self.dcs
                    .context()
                    .PSSetShaderResources(0, Some(&[None, None, None, None, None]));
            }
        }
    }

    fn run_final(&self) {
        unsafe {
            self.scope_alk_composite.bind(0, TfxShaderStage::Vertex);
            self.scope_alk_composite.bind(0, TfxShaderStage::Pixel);
            self.dcs.context().OMSetBlendState(
                &self.blend_state_none,
                Some(&[1f32, 1., 1., 1.] as _),
                0xffffffff,
            );

            self.dcs.context().OMSetRenderTargets(
                Some(&[Some(
                    self.dcs.swapchain_target.read().as_ref().unwrap().clone(),
                )]),
                None,
            );
            self.dcs.context().PSSetShaderResources(
                0,
                Some(&[
                    Some(self.gbuffer.staging.view.clone()),
                    Some(self.gbuffer.depth.texture_view.clone()),
                ]),
            );

            self.dcs.context().RSSetScissorRects(None);
            self.dcs.context().RSSetViewports(Some(&[D3D11_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: self.window_size.0 as f32,
                Height: self.window_size.1 as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }]));

            self.dcs.context().VSSetShader(&self.final_vs, None);
            self.dcs.context().PSSetShader(&self.final_ps, None);
            self.dcs
                .context()
                .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
            self.dcs.context().Draw(4, 0);

            self.dcs
                .context()
                .PSSetShaderResources(0, Some(&[None, None, None, None, None]));
        }
    }

    const CAMERA_CASCADE_CLIP_NEAR: f32 = 0.1;
    const CAMERA_CASCADE_CLIP_FAR: f32 = 4000.0;
    const CAMERA_CASCADE_LEVELS: &'static [f32] = &[
        Self::CAMERA_CASCADE_CLIP_FAR / 50.0,
        Self::CAMERA_CASCADE_CLIP_FAR / 25.0,
        Self::CAMERA_CASCADE_CLIP_FAR / 10.0,
        Self::CAMERA_CASCADE_CLIP_FAR / 1.0,
    ];
    const CAMERA_CASCADE_LEVEL_COUNT: usize = Self::CAMERA_CASCADE_LEVELS.len();

    fn update_directional_cascades(&self, resources: &Resources) {
        let camera = resources.get::<FpsCamera>().unwrap();

        let light_dir = resources.get::<RenderSettings>().unwrap().light_dir;

        let mut cascade_matrices = [Mat4::IDENTITY; Self::CAMERA_CASCADE_LEVEL_COUNT];

        let view = camera.calculate_matrix();

        // clippy: Annoying lint, code is harder to read with the lint's suggested method
        #[allow(clippy::needless_range_loop)]
        for i in 0..Self::CAMERA_CASCADE_LEVEL_COUNT {
            let (z_start, z_end) = if i == 0 {
                (
                    Self::CAMERA_CASCADE_CLIP_NEAR,
                    Self::CAMERA_CASCADE_LEVELS[i],
                )
            } else if i < Self::CAMERA_CASCADE_LEVEL_COUNT {
                (
                    Self::CAMERA_CASCADE_LEVELS[i - 1],
                    Self::CAMERA_CASCADE_LEVELS[i],
                )
            } else {
                (
                    Self::CAMERA_CASCADE_LEVELS[i - 1],
                    Self::CAMERA_CASCADE_CLIP_FAR,
                )
            };

            let light_matrix = camera.build_cascade(
                light_dir,
                view,
                z_start,
                z_end,
                self.window_size.0 as f32 / self.window_size.1 as f32,
            );

            cascade_matrices[i] = light_matrix;
        }
        self.scope_alk_cascade_transforms
            .write(&cascade_matrices)
            .unwrap();
        *self.light_cascade_transforms.write() = cascade_matrices;
    }

    fn render_cascade_depthmaps(&self, resources: &Resources) {
        self.update_directional_cascades(resources);

        let shader_overrides = resources.get::<EnabledShaderOverrides>().unwrap();
        let render_settings = resources.get::<RenderSettings>().unwrap();

        let csb = resources.get::<ShadowMapsResource>().unwrap();
        for v in &csb.cascade_depth_buffers.views {
            unsafe {
                self.dcs.context().ClearDepthStencilView(
                    v,
                    (D3D11_CLEAR_DEPTH.0 | D3D11_CLEAR_STENCIL.0) as _,
                    1.0,
                    0,
                );
            }
        }

        if !render_settings.render_shadows {
            return;
        }

        let draw_queue = self.draw_queue.read();

        unsafe {
            let csb = resources.get::<ShadowMapsResource>().unwrap();
            self.dcs.context().RSSetViewports(Some(&[D3D11_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: csb.resolution as f32,
                Height: csb.resolution as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }]));

            // We don't want to cull backfaces for shadow map rendering, otherwise light will be able to leak through unclosed geometry
            self.dcs.context().RSSetState(&self.rasterizer_state_nocull);
        }

        let scope_view_base = self.scope_view_backup.read();
        for cascade_level in 0..Self::CAMERA_CASCADE_LEVEL_COUNT {
            unsafe {
                self.dcs
                    .context()
                    .OMSetDepthStencilState(&csb.cascade_depth_buffers.state, 0);

                let view = &csb.cascade_depth_buffers.views[cascade_level];
                self.dcs.context().ClearDepthStencilView(
                    view,
                    (D3D11_CLEAR_DEPTH.0 | D3D11_CLEAR_STENCIL.0) as _,
                    1.0,
                    0,
                );

                self.dcs.context().OMSetRenderTargets(None, view);
            }

            let mat = self.light_cascade_transforms.read()[cascade_level];
            self.scope_view_csm
                .write(&ScopeView {
                    world_to_projective: mat,
                    camera_to_world: Mat4::default(),
                    view_miscellaneous: mat.w_axis,
                    ..*scope_view_base
                })
                .expect("Failed to write cascade scope_view");
            self.scope_view_csm.bind(12, TfxShaderStage::Vertex);
            self.scope_view_csm.bind(12, TfxShaderStage::Pixel);

            for i in 0..draw_queue.len() {
                if !draw_queue[i].0.transparency().should_write_depth() {
                    continue;
                }

                let (s, d) = draw_queue[i].clone();
                self.draw(s, &d, &shader_overrides, DrawMode::DepthPrepass, false);
            }
        }
    }

    fn update_buffers(&self, resources: &Resources) -> anyhow::Result<()> {
        let mut camera = resources.get_mut::<FpsCamera>().unwrap();
        let overrides = resources.get::<ScopeOverrides>().unwrap();

        self.scope_frame.write(&ScopeFrame {
            game_time: self.start_time.elapsed().as_secs_f32(),
            render_time: self.start_time.elapsed().as_secs_f32(),
            delta_game_time: *self.delta_time.read(),
            // exposure_time: 0.0,

            // exposure_scale: 1.0,
            // exposure_illum_relative_glow: 1.0,
            // exposure_scale_for_shading: 1.0,
            // exposure_illum_relative: 1.0,
            // random_seed_scales: Vec4::ONE,
            // overrides: Vec4::splat(0.5),

            // unk4: Vec4::ONE,
            // unk5: Vec4::ONE,
            // unk6: Vec4::ONE,
            // unk7: Vec4::ONE,
            ..overrides.frame
        })?;

        camera.projection_matrix = Mat4::perspective_infinite_reverse_rh(
            90f32.to_radians(),
            self.window_size.0 as f32 / self.window_size.1 as f32,
            0.0001,
        );

        let view: Mat4 = camera.calculate_matrix();
        let world_to_projective = camera.projection_matrix * view;
        let camera_to_world = Mat4::from_translation(camera.position);
        // let camera_to_world = Mat4::from_cols(
        //     view.x_axis,
        //     view.y_axis,
        //     view.z_axis,
        //     camera.position.extend(view.w_axis.w),
        // );

        let scale_x = 2.0 / self.window_size.0 as f32;
        let scale_y = -2.0 / self.window_size.1 as f32;
        let viewport_inv = Mat4::from_cols_array_2d(&[
            [scale_x, 0.0, 0.0, 0.0],
            [0.0, scale_y, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0, 1.0],
        ]);

        *self.camera_viewproj.write() = camera.projection_matrix * view;
        *self.camera_svp_inv.write() = (camera.projection_matrix * view).inverse() * viewport_inv;

        let scope_view_data = ScopeView {
            world_to_projective,
            camera_to_world,

            // camera_right: camera.right.extend(1.0),
            // camera_up: camera.up.extend(1.0),
            // camera_backward: -camera.front.extend(1.0),
            // camera_position: camera.position.extend(1.0),
            target_pixel_to_camera: Mat4::from_scale_rotation_translation(
                Vec3::new(2.0, 1.0, 1.0),
                Quat::IDENTITY,
                Vec3::new(2.0, 1.0, 1.0) * 100.0,
            )
            .inverse(),

            target_resolution: (self.window_size.0 as f32, self.window_size.1 as f32),
            inverse_target_resolution: (
                (1. / (self.window_size.0 as f32)),
                (1. / (self.window_size.1 as f32)),
            ),
            // Z value accounts for missing depth value
            view_miscellaneous: Vec4::new(0.0, 0.0, 0.0001, 0.0),
            // maximum_depth_pre_projection: 0.0, // TODO
            // view_is_first_person: 0.0,
            // misc_unk2: 0.0001,
            // misc_unk3: 0.0,
            ..overrides.view
        };

        let scope_view_pixel_data = ScopeView {
            view_miscellaneous: camera.position.extend(1.0),
            ..scope_view_data
        };

        self.scope_view.write(&scope_view_data)?;
        self.scope_view_pixel.write(&scope_view_pixel_data)?;
        *self.scope_view_backup.write() = scope_view_data;

        self.scope_unk2.write(&overrides.unk2)?;
        self.scope_unk3.write(&overrides.unk3)?;
        self.scope_unk8.write(&overrides.unk8)?;

        Ok(())
    }

    pub fn resize(&mut self, new_size: (u32, u32)) -> anyhow::Result<()> {
        self.window_size = new_size;
        self.gbuffer.resize(new_size)
    }

    pub fn clear_render_targets(&self) {
        unsafe {
            self.dcs.context().ClearRenderTargetView(
                &self.gbuffer.rt0.render_target,
                [0.0, 0.0, 0.0, 0.0].as_ptr() as _,
            );
            self.dcs.context().ClearRenderTargetView(
                &self.gbuffer.rt1.render_target,
                [0.0, 0.0, 0.0, 0.0].as_ptr() as _,
            );
            self.dcs.context().ClearRenderTargetView(
                &self.gbuffer.rt2.render_target,
                [1.0, 0.5, 1.0, 0.0].as_ptr() as _,
            );
            self.dcs.context().ClearDepthStencilView(
                &self.gbuffer.depth.view,
                D3D11_CLEAR_DEPTH.0 as _,
                0.0,
                0,
            );
        }
    }
}

#[derive(Default, PartialEq)]
pub enum DrawMode {
    #[default]
    Normal = 0,
    DepthPrepass = 1,
}

pub struct ShadowMapsResource {
    pub cascade_depth_buffers: ShadowDepthMap,
    pub resolution: usize,
    dcs: Arc<DeviceContextSwapchain>,
}

impl ShadowMapsResource {
    pub const DEFAULT_RESOLUTION: usize = 4096;

    pub fn create(dcs: Arc<DeviceContextSwapchain>) -> Self {
        Self {
            cascade_depth_buffers: ShadowDepthMap::create(
                (Self::DEFAULT_RESOLUTION as _, Self::DEFAULT_RESOLUTION as _),
                Renderer::CAMERA_CASCADE_LEVEL_COUNT,
                &dcs.device,
            )
            .expect("Failed to create CSM depth map"),
            resolution: Self::DEFAULT_RESOLUTION,
            dcs,
        }
    }

    pub fn resize(&mut self, new_resolution: usize) {
        if new_resolution == self.resolution {
            return;
        }

        self.cascade_depth_buffers
            .resize(
                (new_resolution as u32, new_resolution as u32),
                &self.dcs.device,
            )
            .expect("Failed to resize shadow map depth buffer");
        self.resolution = new_resolution;

        info!("Resized shadow maps to {new_resolution}");
    }
}
