use crate::{
    backup::BackupState,
    mesh::{create_index_buffer, create_vertex_buffer, GpuMesh, GpuVertex},
    shader::CompiledShaders,
    texture::TextureAllocator,
    RenderError,
};
use egui::{epaint::Primitive, Context};
use std::mem::size_of;
use windows::{
    core::HRESULT,
    s,
    Win32::{
        Foundation::{HWND, RECT},
        Graphics::{
            Direct3D::D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
            Direct3D11::{
                ID3D11Device, ID3D11DeviceContext, ID3D11InputLayout, ID3D11RenderTargetView,
                ID3D11Texture2D, D3D11_APPEND_ALIGNED_ELEMENT, D3D11_BLEND_DESC,
                D3D11_BLEND_INV_SRC_ALPHA, D3D11_BLEND_ONE, D3D11_BLEND_OP_ADD,
                D3D11_BLEND_SRC_ALPHA, D3D11_COLOR_WRITE_ENABLE_ALL, D3D11_COMPARISON_ALWAYS,
                D3D11_CULL_NONE, D3D11_FILL_SOLID, D3D11_FILTER_MIN_MAG_MIP_LINEAR,
                D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_VERTEX_DATA, D3D11_RASTERIZER_DESC,
                D3D11_RENDER_TARGET_BLEND_DESC, D3D11_SAMPLER_DESC, D3D11_TEXTURE_ADDRESS_BORDER,
                D3D11_VIEWPORT,
            },
            Dxgi::{
                Common::{
                    DXGI_FORMAT_R32G32B32A32_FLOAT, DXGI_FORMAT_R32G32_FLOAT, DXGI_FORMAT_R32_UINT,
                },
                IDXGISwapChain, DXGI_SWAP_CHAIN_DESC,
            },
        },
        UI::WindowsAndMessaging::GetClientRect,
    },
};

/// Heart and soul of this integration.
/// Main methods you are going to use are:
/// * [`Self::present`] - Should be called inside of hook or before present.
/// * [`Self::resize_buffers`] - Should be called **INSTEAD** of swapchain's `ResizeBuffers`.
/// * [`Self::wnd_proc`] - Should be called on each `WndProc`.
pub struct DirectX11Renderer {
    render_view: Option<ID3D11RenderTargetView>,
    tex_alloc: TextureAllocator,
    input_layout: ID3D11InputLayout,
    shaders: CompiledShaders,
    backup: BackupState,
    hwnd: HWND,
}

impl DirectX11Renderer {
    const INPUT_ELEMENTS_DESC: [D3D11_INPUT_ELEMENT_DESC; 3] = [
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: s!("POSITION"),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: 0,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: s!("TEXCOORD"),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: s!("COLOR"),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
    ];
}

impl DirectX11Renderer {
    /// Create a new directx11 renderer from a swapchain
    pub fn init_from_swapchain(swapchain: &IDXGISwapChain) -> Result<Self, RenderError> {
        unsafe {
            let mut swap_chain_desc = DXGI_SWAP_CHAIN_DESC::default();
            swapchain.GetDesc(&mut swap_chain_desc)?;

            let hwnd = swap_chain_desc.OutputWindow;
            if hwnd.0 == -1 {
                return Err(RenderError::General(
                    "Trying to initialize from a swapchain with an invalid hwnd",
                ));
            }
            let dev: ID3D11Device = swapchain.GetDevice()?;
            let backbuffer: ID3D11Texture2D = swapchain.GetBuffer(0)?;

            let mut render_view = None;
            dev.CreateRenderTargetView(&backbuffer, None, Some(&mut render_view))?;

            let shaders = CompiledShaders::new(&dev)?;
            let mut input_layout = None;
            dev.CreateInputLayout(
                &Self::INPUT_ELEMENTS_DESC,
                shaders.bytecode(),
                Some(&mut input_layout),
            )?;
            let input_layout =
                input_layout.ok_or(RenderError::General("failed to initialize input layout"))?;

            Ok(Self {
                tex_alloc: TextureAllocator::default(),
                backup: BackupState::default(),
                input_layout,
                render_view,
                shaders,
                hwnd,
            })
        }
    }
}

impl DirectX11Renderer {
    /// Present call. Should be called once per original present call, before or inside of hook.
    #[allow(clippy::cast_ref_to_mut)]
    pub fn paint<PaintFn>(
        &mut self,
        swap_chain: &IDXGISwapChain,
        input: egui::RawInput,
        context: &Context,
        paint: PaintFn,
    ) -> Result<(), RenderError>
    where
        PaintFn: FnOnce(&Context),
    {
        unsafe {
            let (dev, ctx) = &get_device_and_context(swap_chain)?;
            self.backup.save(ctx);
            let screen = self.get_screen_size();
            let output = context.run(input, paint);

            if !output.textures_delta.is_empty() {
                self.tex_alloc
                    .process_deltas(dev, ctx, output.textures_delta)?;
            }

            if output.shapes.is_empty() {
                self.backup.restore(ctx);
                return Ok(());
            }

            let primitives = context
                .tessellate(output.shapes)
                .into_iter()
                .filter_map(|prim| {
                    if let Primitive::Mesh(mesh) = prim.primitive {
                        GpuMesh::from_mesh(screen, mesh, prim.clip_rect)
                    } else {
                        panic!("Paint callbacks are not yet supported")
                    }
                })
                .collect::<Vec<_>>();

            self.set_blend_state(dev, ctx)?;
            self.set_raster_options(dev, ctx)?;
            self.set_sampler_state(dev, ctx)?;

            ctx.RSSetViewports(Some(&[self.get_viewport()]));
            ctx.OMSetRenderTargets(Some(&[self.render_view.clone()]), None);
            ctx.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            ctx.IASetInputLayout(&self.input_layout);

            for mesh in primitives {
                let idx = create_index_buffer(dev, &mesh)?;
                let vtx = create_vertex_buffer(dev, &mesh)?;

                let texture = self.tex_alloc.get_by_id(mesh.texture_id);

                ctx.RSSetScissorRects(Some(&[RECT {
                    left: mesh.clip.left() as _,
                    top: mesh.clip.top() as _,
                    right: mesh.clip.right() as _,
                    bottom: mesh.clip.bottom() as _,
                }]));

                if texture.is_some() {
                    ctx.PSSetShaderResources(0, Some(&[texture]));
                }

                ctx.IASetVertexBuffers(
                    0,
                    1,
                    Some(&Some(vtx)),
                    Some(&(size_of::<GpuVertex>() as _)),
                    Some(&0),
                );
                ctx.IASetIndexBuffer(&idx, DXGI_FORMAT_R32_UINT, 0);
                ctx.VSSetShader(&self.shaders.vertex, Some(&[]));
                ctx.PSSetShader(&self.shaders.pixel, Some(&[]));

                ctx.DrawIndexed(mesh.indices.len() as _, 0, 0);
            }

            self.backup.restore(ctx);
        }

        Ok(())
    }

    /// Call when resizing buffers.
    /// Do not call the original function before it, instead call it inside of the `original` closure.
    /// # Behavior
    /// In `origin` closure make sure to call the original `ResizeBuffers`.
    pub fn resize_buffers(
        &mut self,
        swap_chain: &IDXGISwapChain,
        original: impl FnOnce() -> HRESULT,
    ) -> Result<HRESULT, RenderError> {
        unsafe {
            drop(self.render_view.take());
            let result = original();
            let backbuffer: ID3D11Texture2D = swap_chain.GetBuffer(0)?;
            let device: ID3D11Device = swap_chain.GetDevice()?;
            device.CreateRenderTargetView(&backbuffer, None, Some(&mut self.render_view))?;
            Ok(result)
        }
    }
}

impl DirectX11Renderer {
    #[inline]
    fn get_screen_size(&self) -> (f32, f32) {
        let mut rect = RECT::default();
        unsafe {
            GetClientRect(self.hwnd, &mut rect);
        }
        (
            (rect.right - rect.left) as f32,
            (rect.bottom - rect.top) as f32,
        )
    }

    #[inline]
    fn get_viewport(&self) -> D3D11_VIEWPORT {
        let (w, h) = self.get_screen_size();
        D3D11_VIEWPORT {
            TopLeftX: 0.,
            TopLeftY: 0.,
            Width: w,
            Height: h,
            MinDepth: 0.,
            MaxDepth: 1.,
        }
    }

    fn set_blend_state(
        &self,
        dev: &ID3D11Device,
        ctx: &ID3D11DeviceContext,
    ) -> Result<(), RenderError> {
        let mut targets: [D3D11_RENDER_TARGET_BLEND_DESC; 8] = Default::default();
        targets[0].BlendEnable = true.into();
        targets[0].SrcBlend = D3D11_BLEND_SRC_ALPHA;
        targets[0].DestBlend = D3D11_BLEND_INV_SRC_ALPHA;
        targets[0].BlendOp = D3D11_BLEND_OP_ADD;
        targets[0].SrcBlendAlpha = D3D11_BLEND_ONE;
        targets[0].DestBlendAlpha = D3D11_BLEND_INV_SRC_ALPHA;
        targets[0].BlendOpAlpha = D3D11_BLEND_OP_ADD;
        targets[0].RenderTargetWriteMask = D3D11_COLOR_WRITE_ENABLE_ALL.0 as _;

        let blend_desc = D3D11_BLEND_DESC {
            AlphaToCoverageEnable: false.into(),
            IndependentBlendEnable: false.into(),
            RenderTarget: targets,
        };

        unsafe {
            let mut blend_state = None;
            dev.CreateBlendState(&blend_desc, Some(&mut blend_state))?;
            let blend_state =
                blend_state.ok_or(RenderError::General("Unable to set blend state"))?;
            ctx.OMSetBlendState(&blend_state, Some([0., 0., 0., 0.].as_ptr()), 0xffffffff);
        }

        Ok(())
    }

    fn set_raster_options(
        &self,
        dev: &ID3D11Device,
        ctx: &ID3D11DeviceContext,
    ) -> Result<(), RenderError> {
        let raster_desc = D3D11_RASTERIZER_DESC {
            FillMode: D3D11_FILL_SOLID,
            CullMode: D3D11_CULL_NONE,
            FrontCounterClockwise: false.into(),
            DepthBias: false.into(),
            DepthBiasClamp: 0.,
            SlopeScaledDepthBias: 0.,
            DepthClipEnable: false.into(),
            ScissorEnable: true.into(),
            MultisampleEnable: false.into(),
            AntialiasedLineEnable: false.into(),
        };

        unsafe {
            let mut options = None;
            dev.CreateRasterizerState(&raster_desc, Some(&mut options))?;
            let options = options.ok_or(RenderError::General("Unable to set options"))?;
            ctx.RSSetState(&options);
            Ok(())
        }
    }

    fn set_sampler_state(
        &self,
        dev: &ID3D11Device,
        ctx: &ID3D11DeviceContext,
    ) -> Result<(), RenderError> {
        let desc = D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_BORDER,
            AddressV: D3D11_TEXTURE_ADDRESS_BORDER,
            AddressW: D3D11_TEXTURE_ADDRESS_BORDER,
            MipLODBias: 0.,
            ComparisonFunc: D3D11_COMPARISON_ALWAYS,
            MinLOD: 0.,
            MaxLOD: 0.,
            BorderColor: [1., 1., 1., 1.],
            ..Default::default()
        };

        unsafe {
            let mut sampler = None;
            dev.CreateSamplerState(&desc, Some(&mut sampler))?;
            ctx.PSSetSamplers(0, Some(&[sampler]));
            Ok(())
        }
    }
}

unsafe fn get_device_and_context(
    swap: &IDXGISwapChain,
) -> Result<(ID3D11Device, ID3D11DeviceContext), RenderError> {
    let device: ID3D11Device = swap.GetDevice()?;
    let context = device.GetImmediateContext()?;
    Ok((device, context))
}
