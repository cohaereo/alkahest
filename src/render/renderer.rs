use std::{rc::Rc, time::Instant};

use anyhow::Context;
use glam::{Mat4, Vec4};
use windows::Win32::Graphics::Direct3D::Fxc::{
    D3DCompileFromFile, D3DCOMPILE_DEBUG, D3DCOMPILE_SKIP_OPTIMIZATION,
};
use windows::Win32::Graphics::Direct3D::D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP;
use windows::Win32::Graphics::Direct3D11::*;
use winit::window::Window;

use crate::dxgi::DxgiFormat;
use crate::overlays::camera_settings::CurrentCubemap;
use crate::overlays::render_settings::CompositorOptions;
use crate::render::drawcall::ShaderStages;
use crate::render::scopes::ScopeUnk2;
use crate::render::RenderData;
use crate::texture::Texture;
use crate::{camera::FpsCamera, resources::Resources};

use super::drawcall::Transparency;
use super::scopes::ScopeUnk8;
use super::{
    drawcall::{DrawCall, ShadingTechnique, SortValue3d},
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

pub struct Renderer {
    draw_queue: Vec<(SortValue3d, DrawCall)>,

    state: RendererState,
    pub gbuffer: GBuffer,
    window_size: (u32, u32),
    dcs: Rc<DeviceContextSwapchain>,

    scope_view: ConstantBuffer<ScopeView>,
    scope_frame: ConstantBuffer<ScopeFrame>,
    scope_unk2: ConstantBuffer<ScopeUnk2>,
    scope_unk8: ConstantBuffer<ScopeUnk8>,
    scope_alk_composite: ConstantBuffer<CompositorOptions>,

    start_time: Instant,
    last_frame: Instant,
    delta_time: f32,

    pub render_data: RenderData,

    blend_state_none: ID3D11BlendState,
    blend_state_blend: ID3D11BlendState,
    blend_state_additive: ID3D11BlendState,

    rasterizer_state: ID3D11RasterizerState,
    rasterizer_state_nocull: ID3D11RasterizerState,

    matcap: Texture,
    // A 4x4 white texture
    white: Texture,
    // A 4x4 mid-grey texture
    gray: Texture,

    composite_vs: ID3D11VertexShader,
    composite_ps: ID3D11PixelShader,

    final_vs: ID3D11VertexShader,
    final_ps: ID3D11PixelShader,
}

impl Renderer {
    pub fn create(window: &Window, dcs: Rc<DeviceContextSwapchain>) -> anyhow::Result<Self> {
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

        const MATCAP_DATA: &[u8] = include_bytes!("../../assets/textures/matte.data");
        let matcap = Texture::load_2d_raw(
            &dcs,
            128,
            128,
            MATCAP_DATA,
            DxgiFormat::R8G8B8A8_UNORM,
            Some("Basic shading matcap"),
        )?;

        let white = Texture::load_2d_raw(
            &dcs,
            4,
            4,
            &[0xffu8; 4 * 4 * 4],
            DxgiFormat::R8G8B8A8_UNORM,
            Some("4x4 white"),
        )?;

        let gray = Texture::load_2d_raw(
            &dcs,
            4,
            4,
            &[0x7fu8; 4 * 4 * 4],
            DxgiFormat::R8G8B8A8_UNORM,
            Some("4x4 white"),
        )?;

        let mut vshader_composite = None;
        let mut pshader_composite = None;
        let mut errors = None;

        let flags = if cfg!(debug_assertions) {
            D3DCOMPILE_DEBUG | D3DCOMPILE_SKIP_OPTIMIZATION
        } else {
            0
        };
        unsafe {
            (
                D3DCompileFromFile(
                    w!("assets/shaders/composite.hlsl"),
                    None,
                    None,
                    s!("VShader"),
                    s!("vs_5_0"),
                    flags,
                    0,
                    &mut vshader_composite,
                    Some(&mut errors),
                )
                .context("Failed to compile vertex shader")?,
                D3DCompileFromFile(
                    w!("assets/shaders/composite.hlsl"),
                    None,
                    None,
                    s!("PShader"),
                    s!("ps_5_0"),
                    flags,
                    0,
                    &mut pshader_composite,
                    Some(&mut errors),
                )
                .context("Failed to compile pixel shader")?,
            )
        };

        if let Some(errors) = errors {
            let estr = unsafe {
                let eptr = errors.GetBufferPointer();
                std::slice::from_raw_parts(eptr.cast(), errors.GetBufferSize())
            };
            let errors = String::from_utf8_lossy(estr);
            warn!("{}", errors);
        }

        let vshader_composite = vshader_composite.unwrap();
        let pshader_composite = pshader_composite.unwrap();

        let (vshader_composite, pshader_composite) = unsafe {
            let vs_blob = std::slice::from_raw_parts(
                vshader_composite.GetBufferPointer() as *const u8,
                vshader_composite.GetBufferSize(),
            );
            let v2 = dcs.device.CreateVertexShader(vs_blob, None)?;
            let ps_blob = std::slice::from_raw_parts(
                pshader_composite.GetBufferPointer() as *const u8,
                pshader_composite.GetBufferSize(),
            );
            let v3 = dcs.device.CreatePixelShader(ps_blob, None)?;
            (v2, v3)
        };

        let mut vshader_final = None;
        let mut pshader_final = None;
        let mut errors = None;

        let flags = if cfg!(debug_assertions) {
            D3DCOMPILE_DEBUG | D3DCOMPILE_SKIP_OPTIMIZATION
        } else {
            0
        };
        unsafe {
            (
                D3DCompileFromFile(
                    w!("assets/shaders/final.hlsl"),
                    None,
                    None,
                    s!("VShader"),
                    s!("vs_5_0"),
                    flags,
                    0,
                    &mut vshader_final,
                    Some(&mut errors),
                )
                .context("Failed to compile vertex shader")?,
                D3DCompileFromFile(
                    w!("assets/shaders/final.hlsl"),
                    None,
                    None,
                    s!("PShader"),
                    s!("ps_5_0"),
                    flags,
                    0,
                    &mut pshader_final,
                    Some(&mut errors),
                )
                .context("Failed to compile pixel shader")?,
            )
        };

        if let Some(errors) = errors {
            let estr = unsafe {
                let eptr = errors.GetBufferPointer();
                std::slice::from_raw_parts(eptr.cast(), errors.GetBufferSize())
            };
            let errors = String::from_utf8_lossy(estr);
            warn!("{}", errors);
        }

        let vshader_final = vshader_final.unwrap();
        let pshader_final = pshader_final.unwrap();

        let (vshader_final, pshader_final) = unsafe {
            let vs_blob = std::slice::from_raw_parts(
                vshader_final.GetBufferPointer() as *const u8,
                vshader_final.GetBufferSize(),
            );
            let v2 = dcs.device.CreateVertexShader(vs_blob, None)?;
            let ps_blob = std::slice::from_raw_parts(
                pshader_final.GetBufferPointer() as *const u8,
                pshader_final.GetBufferSize(),
            );
            let v3 = dcs.device.CreatePixelShader(ps_blob, None)?;
            (v2, v3)
        };

        Ok(Renderer {
            draw_queue: Vec::with_capacity(8192),
            state: RendererState::Awaiting,
            gbuffer: GBuffer::create(
                (window.inner_size().width, window.inner_size().height),
                dcs.clone(),
            )?,
            window_size: (window.inner_size().width, window.inner_size().height),
            scope_frame: ConstantBuffer::create(dcs.clone(), None)?,
            scope_view: ConstantBuffer::create(dcs.clone(), None)?,
            scope_unk2: ConstantBuffer::create(dcs.clone(), None)?,
            scope_unk8: ConstantBuffer::create(dcs.clone(), None)?,
            scope_alk_composite: ConstantBuffer::create(dcs.clone(), None)?,
            dcs,
            start_time: Instant::now(),
            last_frame: Instant::now(),
            delta_time: 0.016,
            render_data: RenderData {
                materials: Default::default(),
                vshaders: Default::default(),
                pshaders: Default::default(),
                cbuffers_vs: Default::default(),
                cbuffers_ps: Default::default(),
                textures: Default::default(),
                samplers: Default::default(),
            },
            blend_state_none,
            blend_state_blend,
            blend_state_additive,
            rasterizer_state,
            rasterizer_state_nocull,
            matcap,
            white,
            gray,
            composite_vs: vshader_composite,
            composite_ps: pshader_composite,
            final_vs: vshader_final,
            final_ps: pshader_final,
        })
    }

    pub fn begin_frame(&mut self) {
        if self.state == RendererState::Recording {
            panic!("Called begin(), but a frame is already being recorded! Did you call submit()?")
        }

        self.delta_time = self.last_frame.elapsed().as_secs_f32();
        self.last_frame = Instant::now();

        self.draw_queue.clear();
        self.state = RendererState::Recording;
    }

    // TODO(cohae): `begin` should probably return a CommandEncoder that we can record stuff in
    pub fn push_drawcall(&mut self, ordering: SortValue3d, drawcall: DrawCall) {
        self.draw_queue.push((ordering, drawcall))
    }

    /// Submits recorded drawcalls
    pub fn submit_frame(
        &mut self,
        resources: &Resources,
        draw_lights: bool,
        alpha_blending: bool,
        compositor_mode: usize,
        blend_override: usize,
        lights: (ID3D11Buffer, usize),
    ) {
        if self.state != RendererState::Recording {
            panic!("Called submit(), but the renderer is not recording! Did you call begin()?")
        }

        self.draw_queue
            .sort_unstable_by(|(o1, _), (o2, _)| o1.cmp(o2));

        self.update_buffers(resources)
            .expect("Renderer::update_buffers");

        self.scope_unk2.bind(2, ShaderStages::all());
        self.scope_unk8.bind(8, ShaderStages::all());
        self.scope_view.bind(12, ShaderStages::all());
        self.scope_frame.bind(13, ShaderStages::all());

        unsafe {
            self.dcs.context.OMSetRenderTargets(
                Some(&[
                    Some(self.gbuffer.rt0.render_target.clone()),
                    Some(self.gbuffer.rt1.render_target.clone()),
                    Some(self.gbuffer.rt2.render_target.clone()),
                ]),
                &self.gbuffer.depth.view,
            );
            self.dcs
                .context
                .OMSetDepthStencilState(&self.gbuffer.depth.state, 0);
            self.dcs.context.OMSetBlendState(
                &self.blend_state_none,
                Some(&[1f32, 1., 1., 1.] as _),
                0xffffffff,
            );
        }

        //region Deferred
        for i in 0..self.draw_queue.len() {
            if self.draw_queue[i].0.technique() != ShadingTechnique::Deferred {
                continue;
            }

            let (s, d) = self.draw_queue[i].clone();
            self.draw(s, &d);
        }
        //endregion

        self.run_deferred_shading(resources, draw_lights, compositor_mode, lights);

        unsafe {
            self.dcs
                .context
                .OMSetDepthStencilState(&self.gbuffer.depth.state, 0);

            self.dcs.context.OMSetRenderTargets(
                Some(&[Some(
                    self.gbuffer.staging.render_target.clone(), // self.dcs.swapchain_target.read().as_ref().unwrap().clone(),
                )]),
                &self.gbuffer.depth.view,
            );

            let rt = self.gbuffer.staging.view.clone();
            self.dcs.context.PSSetShaderResources(
                12,
                Some(&[
                    // TODO(cohae): Totally wrong, obviously
                    Some(rt.clone()),
                    Some(rt.clone()),
                    Some(rt.clone()),
                    Some(rt.clone()),
                    Some(rt.clone()),
                    Some(rt),
                ]),
            );
        }

        //region Forward
        let mut transparency_mode = Transparency::None;
        for i in 0..self.draw_queue.len() {
            unsafe {
                self.dcs.context.PSSetShaderResources(
                    10,
                    Some(&[Some(self.gbuffer.depth.texture_view.clone())]),
                );
            }

            for slot in 11..18 {
                self.white.bind(&self.dcs, slot, ShaderStages::all());
            }

            if self.draw_queue[i].0.technique() != ShadingTechnique::Forward {
                continue;
            }

            let (s, d) = self.draw_queue[i].clone();
            if s.transparency() != transparency_mode {
                if alpha_blending {
                    // Swap to read-only depth state once we start rendering translucent geometry
                    if s.transparency() != Transparency::None
                        && s.transparency() != Transparency::Cutout
                    {
                        unsafe {
                            self.dcs
                                .context
                                .OMSetDepthStencilState(&self.gbuffer.depth.state_readonly, 0);
                        }
                    }

                    unsafe {
                        match blend_override {
                            1 => self.dcs.context.OMSetBlendState(
                                &self.blend_state_blend,
                                Some(&[1f32, 1., 1., 1.] as _),
                                0xffffffff,
                            ),
                            2 => self.dcs.context.OMSetBlendState(
                                &self.blend_state_additive,
                                Some(&[1f32, 1., 1., 1.] as _),
                                0xffffffff,
                            ),
                            _ => match s.transparency() {
                                Transparency::Blend => self.dcs.context.OMSetBlendState(
                                    &self.blend_state_blend,
                                    Some(&[1f32, 1., 1., 1.] as _),
                                    0xffffffff,
                                ),

                                Transparency::Additive => self.dcs.context.OMSetBlendState(
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

            self.draw(s, &d);
        }
        //endregion

        self.run_final();

        self.state = RendererState::Awaiting;
    }

    fn draw(&mut self, sort: SortValue3d, drawcall: &DrawCall) {
        if let Some(mat) = self.render_data.materials.get(&sort.material()) {
            if mat.unk8 != 1 {
                return;
            }

            unsafe {
                if mat.unk22 != 0 {
                    self.dcs.context.RSSetState(&self.rasterizer_state_nocull);
                } else {
                    self.dcs.context.RSSetState(&self.rasterizer_state);
                }
            }

            // TODO(cohae): How can we handle these errors?
            if mat.bind(&self.dcs, &self.render_data).is_err() {
                return;
            }
        } else {
            return;
        }

        unsafe {
            self.dcs
                .context
                .VSSetConstantBuffers(11, Some(&[drawcall.cb11.clone()]));

            self.dcs.context.IASetVertexBuffers(
                0,
                1,
                Some([Some(drawcall.vertex_buffer.clone())].as_ptr()),
                Some([drawcall.vertex_buffer_stride].as_ptr()),
                Some(&0),
            );

            self.dcs.context.IASetIndexBuffer(
                Some(&drawcall.index_buffer),
                drawcall.index_format,
                0,
            );

            self.dcs
                .context
                .IASetPrimitiveTopology(drawcall.primitive_type);

            if drawcall.instance_start.is_some() || drawcall.instance_count.is_some() {
                self.dcs.context.DrawIndexedInstanced(
                    drawcall.index_count,
                    drawcall.instance_count.unwrap_or(1) as _,
                    drawcall.index_start,
                    0,
                    drawcall.instance_start.unwrap_or(0),
                );
            } else {
                self.dcs
                    .context
                    .DrawIndexed(drawcall.index_count, drawcall.index_start, 0);
            }
        }
    }

    /// Swaps to primary swapchain render target, binds gbuffers and runs the shading passes
    fn run_deferred_shading(
        &mut self,
        resources: &Resources,
        draw_lights: bool,
        compositor_mode: usize,
        lights: (ID3D11Buffer, usize),
    ) {
        unsafe {
            self.dcs.context.OMSetBlendState(
                &self.blend_state_none,
                Some(&[1f32, 1., 1., 1.] as _),
                0xffffffff,
            );

            self.dcs.context.OMSetRenderTargets(
                Some(&[Some(self.gbuffer.staging.render_target.clone())]),
                None,
            );
            self.dcs.context.PSSetShaderResources(
                0,
                Some(&[
                    Some(self.gbuffer.rt0.view.clone()),
                    Some(self.gbuffer.rt1.view.clone()),
                    Some(self.gbuffer.rt2.view.clone()),
                    Some(self.gbuffer.depth.texture_view.clone()),
                ]),
            );

            self.matcap.bind(&self.dcs, 4, ShaderStages::PIXEL);

            let cubemap_texture = resources
                .get::<CurrentCubemap>()
                .unwrap()
                .1
                .and_then(|t| self.render_data.textures.get(&t.0).map(|t| t.view.clone()));

            self.dcs
                .context
                .PSSetShaderResources(5, Some(&[cubemap_texture]));

            {
                let mut camera = resources.get_mut::<FpsCamera>().unwrap();
                let projection = Mat4::perspective_infinite_reverse_rh(
                    90f32.to_radians(),
                    self.window_size.0 as f32 / self.window_size.1 as f32,
                    0.0001,
                );

                let view = camera.calculate_matrix();
                let proj_view = projection * view;
                let view = Mat4::from_translation(camera.position);
                let compositor_options = CompositorOptions {
                    proj_view_matrix_inv: proj_view.inverse(),
                    proj_matrix: projection,
                    view_matrix: view,
                    camera_pos: camera.position.extend(1.0),
                    camera_dir: camera.front.extend(1.0),
                    time: self.start_time.elapsed().as_secs_f32(),
                    mode: compositor_mode as u32,
                    light_count: if draw_lights { lights.1 as u32 } else { 0 },
                };
                self.scope_alk_composite.write(&compositor_options).unwrap();
                self.scope_alk_composite.bind(0, ShaderStages::all());
            }
            // cb_composite_options.write(&compositor_options).unwrap();

            // self.dcs
            //     .context
            //     .VSSetConstantBuffers(0, Some(&[Some(cb_composite_options.buffer().clone())]));

            self.dcs
                .context
                .PSSetConstantBuffers(1, Some(&[Some(lights.0)]));

            self.dcs.context.RSSetViewports(Some(&[D3D11_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: self.window_size.0 as f32,
                Height: self.window_size.1 as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }]));

            // self.dcs
            //     .context
            //     .PSSetShaderResources(5, Some(&[cubemap_texture]));

            // self.dcs
            //     .context
            //     .PSSetSamplers(0, Some(&[Some(le_sampler.clone())]));

            self.dcs.context.VSSetShader(&self.composite_vs, None);
            self.dcs.context.PSSetShader(&self.composite_ps, None);
            self.dcs
                .context
                .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
            self.dcs.context.Draw(4, 0);

            self.dcs
                .context
                .PSSetShaderResources(0, Some(&[None, None, None, None, None]));
        }
    }
    fn run_final(&mut self) {
        unsafe {
            self.scope_alk_composite.bind(0, ShaderStages::all());
            self.dcs.context.OMSetBlendState(
                &self.blend_state_none,
                Some(&[1f32, 1., 1., 1.] as _),
                0xffffffff,
            );

            self.dcs.context.OMSetRenderTargets(
                Some(&[Some(
                    self.dcs.swapchain_target.read().as_ref().unwrap().clone(),
                )]),
                None,
            );
            self.dcs
                .context
                .PSSetShaderResources(0, Some(&[Some(self.gbuffer.staging.view.clone())]));

            self.dcs.context.RSSetViewports(Some(&[D3D11_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: self.window_size.0 as f32,
                Height: self.window_size.1 as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }]));

            self.dcs.context.VSSetShader(&self.final_vs, None);
            self.dcs.context.PSSetShader(&self.final_ps, None);
            self.dcs
                .context
                .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
            self.dcs.context.Draw(4, 0);

            self.dcs
                .context
                .PSSetShaderResources(0, Some(&[None, None, None, None, None]));
        }
    }

    fn update_buffers(&mut self, resources: &Resources) -> anyhow::Result<()> {
        let mut camera = resources.get_mut::<FpsCamera>().unwrap();
        self.scope_frame.write(&ScopeFrame {
            game_time: self.start_time.elapsed().as_secs_f32(),
            render_time: self.start_time.elapsed().as_secs_f32(),
            delta_game_time: self.delta_time,
            exposure_time: 0.0,

            exposure_scale: 1.0,
            exposure_illum_relative_glow: 0.0,
            exposure_scale_for_shading: 1.0,
            exposure_illum_relative: 0.0,

            random_seed_scales: Vec4::ZERO,
            overrides: Vec4::ZERO,

            unk4: Vec4::ONE,
            unk5: Vec4::ONE,
            unk6: Vec4::ONE,
            unk7: Vec4::ONE,
        })?;

        let projection = Mat4::perspective_infinite_reverse_rh(
            90f32.to_radians(),
            self.window_size.0 as f32 / self.window_size.1 as f32,
            0.0001,
        );

        let view = camera.calculate_matrix();
        let world_to_projective = projection * view;

        self.scope_view.write(&ScopeView {
            world_to_projective,

            camera_right: camera.right.extend(1.0),
            camera_up: camera.up.extend(1.0),
            camera_backward: -camera.front.extend(1.0),
            camera_position: camera.position.extend(1.0),

            target_pixel_to_camera: Mat4::IDENTITY,
            target_resolution: (self.window_size.0 as f32, self.window_size.1 as f32),
            inverse_target_resolution: (
                // TODO(cohae): Is this correct?
                1. / (self.window_size.0 as f32),
                1. / (self.window_size.1 as f32),
            ),
            maximum_depth_pre_projection: 0.0, // TODO
            view_is_first_person: 0.0,
            // Accounts for missing depth value in vertex output
            misc_unk2: 0.0001,
            misc_unk3: 0.0,
        })?;

        self.scope_unk2.write(&ScopeUnk2 {
            unk0: Vec4::ONE,
            unk1: Vec4::ONE,
            unk2: Vec4::ONE,
            unk3: Vec4::ONE,
            unk4: Vec4::ONE,
            unk5: Vec4::ONE,
        })?;

        self.scope_unk8.write(&ScopeUnk8 {
            unk0: Vec4::ONE,
            unk1: Vec4::ONE,
            unk2: Vec4::ONE,
            unk3: Vec4::ONE,
            unk4: Vec4::ONE,
            unk5: Vec4::ONE,
            unk6: Vec4::ONE,
            unk7: Vec4::ONE,
        })?;

        Ok(())
    }

    pub fn resize(&mut self, new_size: (u32, u32)) -> anyhow::Result<()> {
        self.window_size = new_size;
        self.gbuffer.resize(new_size)
    }

    pub fn clear_render_targets(&mut self) {
        unsafe {
            self.dcs.context.ClearRenderTargetView(
                &self.gbuffer.rt0.render_target,
                [0.5, 0.5, 0.5, 1.0].as_ptr() as _,
            );
            self.dcs.context.ClearRenderTargetView(
                &self.gbuffer.rt1.render_target,
                [0.0, 0.0, 0.0, 0.0].as_ptr() as _,
            );
            self.dcs.context.ClearRenderTargetView(
                &self.gbuffer.rt2.render_target,
                [0.0, 0.0, 0.0, 0.0].as_ptr() as _,
            );
            self.dcs.context.ClearDepthStencilView(
                &self.gbuffer.depth.view,
                D3D11_CLEAR_DEPTH.0 as _,
                0.0,
                0,
            );
        }
    }
}