pub mod buffer;
mod d3dstate;
pub mod debug;
pub mod global_state;
pub mod texture;
pub mod util;

use std::{
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use alkahest_data::{
    dxgi::DxgiFormat, geometry::EPrimitiveType, technique::StateSelection, tfx::TfxShaderStage,
};
use anyhow::Context;
use crossbeam::atomic::AtomicCell;
use debug::PendingGpuTimestampRange;
use parking_lot::{Mutex, ReentrantMutex, ReentrantMutexGuard, RwLock};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::{
    core::Interface,
    Win32::{
        Foundation::{DXGI_STATUS_OCCLUDED, HINSTANCE, HWND},
        Graphics::{
            Direct3D::*,
            Direct3D11::*,
            Dxgi::{Common::*, *},
        },
        UI::WindowsAndMessaging::{SetWindowDisplayAffinity, WINDOW_DISPLAY_AFFINITY},
    },
};

use crate::{
    gpu::{global_state::RenderStates, texture::Texture, util::UtilResources},
    loaders::vertex_buffer::VertexBuffer,
    util::image::Png,
};

pub type SharedGpuContext = Arc<GpuContext>;
pub struct GpuContext {
    pub device: ID3D11Device,
    context: ReentrantMutex<ID3D11DeviceContext>,
    annotation: ID3DUserDefinedAnnotation,

    pub swap_chain: Option<IDXGISwapChain>,
    pub swapchain_target: RwLock<Option<ID3D11RenderTargetView>>,
    pub swapchain_resolution: AtomicCell<(u32, u32)>,

    pub fallback_texture: Texture,
    pub color0_fallback: VertexBuffer,
    pub color_ao_fallback: VertexBuffer,

    pub atmos_depth_angle_lookup_tmp: Texture,
    pub sky_hemisphere_placeholder: Texture,
    pub shadowmap_vs_t2: Texture,
    pub white_texture: Texture,
    pub light_grey_texture: Texture,
    pub grey_texture: Texture,
    pub dark_grey_texture: Texture,
    pub black_texture: Texture,

    pub states: RenderStates,

    present_parameters: AtomicU32,

    current_blend_state: AtomicUsize,
    current_input_layout: AtomicUsize,
    current_rasterizer_state: AtomicUsize,
    current_depth_bias: AtomicUsize,
    current_input_topology: AtomicI32,
    current_depth_state: AtomicUsize,
    use_flipped_depth_comparison: AtomicBool,

    pub current_states: AtomicCell<StateSelection>,

    pub util_resources: UtilResources,
    pub custom_pixel_shader: Option<ID3D11PixelShader>,

    pending_timestamp_queries: Mutex<Vec<PendingGpuTimestampRange>>,
}

const DISPLAY_AFFINITY: WINDOW_DISPLAY_AFFINITY =
    WINDOW_DISPLAY_AFFINITY(0x10FFEF / u16::MAX as u32);
pub static DESKTOP_DISPLAY_MODE: AtomicBool = AtomicBool::new(false);

impl GpuContext {
    pub fn create<Window: HasWindowHandle>(window: &Window) -> anyhow::Result<Self> {
        Self::create_inner(Some(window))
    }

    pub fn create_headless() -> anyhow::Result<Self> {
        Self::create_inner(None::<&winit::window::Window>)
    }

    fn create_device_swapchain<Window: HasWindowHandle>(
        window: Option<&Window>,
    ) -> anyhow::Result<(ID3D11Device, ID3D11DeviceContext, Option<IDXGISwapChain>)> {
        let mut device: Option<ID3D11Device> = None;
        let mut device_context: Option<ID3D11DeviceContext> = None;

        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HINSTANCE::default(),
                Default::default(),
                // D3D11_CREATE_DEVICE_DEBUG,
                Some(&[D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0]),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut device_context),
            )?;
        }

        let device = device.unwrap();
        let device_context = device_context.unwrap();

        let dxgi = unsafe { CreateDXGIFactory::<IDXGIFactory>()? };
        let mut swap_chain: Option<IDXGISwapChain> = None;
        if let Some(window) = window {
            let swap_chain_descriptor: DXGI_SWAP_CHAIN_DESC = {
                let buffer_descriptor = DXGI_MODE_DESC {
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    ..Default::default()
                };

                let sample_descriptor = DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                };

                DXGI_SWAP_CHAIN_DESC {
                    BufferDesc: buffer_descriptor,
                    SampleDesc: sample_descriptor,
                    BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                    BufferCount: 2,
                    OutputWindow: match window.window_handle().unwrap().as_raw() {
                        RawWindowHandle::Win32(h) => HWND(h.hwnd.get()),
                        u => panic!("Can't open window for {u:?}"),
                    },
                    Windowed: true.into(),
                    SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
                    Flags: 0,
                }
            };

            unsafe {
                if !DESKTOP_DISPLAY_MODE.load(Ordering::SeqCst) {
                    // Fixes display issues on certain mobile GPUs
                    SetWindowDisplayAffinity(swap_chain_descriptor.OutputWindow, DISPLAY_AFFINITY)
                        .ok();
                }

                dxgi.CreateSwapChain(&device, &swap_chain_descriptor, &mut swap_chain)
                    .ok()
                    .context("Failed to create swapchain")?;
            }
        }

        Ok((device, device_context, swap_chain))
    }

    fn create_inner<Window: HasWindowHandle>(window: Option<&Window>) -> anyhow::Result<Self> {
        let (device, device_context, swap_chain) = Self::create_device_swapchain(window)?;
        let states = RenderStates::new(&device)?;

        let fallback_texture = Texture::load_png(
            &device,
            &Png::from_bytes(include_bytes!("../../assets/textures/fallback.png"))?,
            Some("Fallback Texture"),
        )?;

        let white_texture = Texture::load_2d_raw(
            &device,
            1,
            1,
            &[255, 255, 255, 255],
            DxgiFormat::R8G8B8A8_UNORM_SRGB,
            Some("White Texture"),
        )?;

        let light_grey_texture = Texture::load_2d_raw(
            &device,
            1,
            1,
            &[196, 196, 196, 196],
            DxgiFormat::R8G8B8A8_UNORM_SRGB,
            Some("Grey Texture"),
        )?;

        let grey_texture = Texture::load_2d_raw(
            &device,
            1,
            1,
            &[127, 127, 127, 127],
            DxgiFormat::R8G8B8A8_UNORM_SRGB,
            Some("Grey Texture"),
        )?;

        let dark_grey_texture = Texture::load_2d_raw(
            &device,
            1,
            1,
            &[40, 40, 40, 127],
            DxgiFormat::R8G8B8A8_UNORM_SRGB,
            Some("Grey Texture"),
        )?;

        let black_texture = Texture::load_2d_raw(
            &device,
            1,
            1,
            &[0, 0, 0, 0],
            DxgiFormat::R8G8B8A8_UNORM_SRGB,
            Some("Black Texture"),
        )?;

        let shadowmap_vs_t2 = Texture::load_2d_raw(
            &device,
            1,
            1,
            &[0, 0, 255, 255],
            DxgiFormat::R8G8B8A8_UNORM_SRGB,
            Some("shadowmap_vs_t2"),
        )?;

        let color0_fallback =
            VertexBuffer::load_data(&device, &[0, 0, 255, 255], 4)?.with_name("color0_fallback");
        let color_ao_fallback =
            VertexBuffer::load_data(&device, &[255; 128 * 4], 4)?.with_name("color_ao_fallback");

        let sky_hemisphere_placeholder = Texture::load_png(
            &device,
            &Png::from_bytes(include_bytes!(
                "../../assets/textures/sky_hemisphere_placeholder.png"
            ))?,
            Some("sky_hemisphere_placeholder.png"),
        )?;

        let atmos_depth_angle_lookup_tmp = Texture::load_png(
            &device,
            &Png::from_bytes(include_bytes!(
                "../../assets/textures/depth_angle_lookup_temp.png"
            ))?,
            Some("depth_angle_lookup_temp.png"),
        )?;

        let mut swapchain_target = None;
        unsafe {
            if let Some(swap_chain) = &swap_chain {
                let buffer = swap_chain.GetBuffer::<ID3D11Resource>(0)?;
                device.CreateRenderTargetView(&buffer, None, Some(&mut swapchain_target))?;
            }
        };

        Ok(Self {
            util_resources: UtilResources::new(&device),

            device,
            annotation: device_context.cast()?,
            context: ReentrantMutex::new(device_context),

            swap_chain,
            swapchain_target: RwLock::new(swapchain_target),
            present_parameters: AtomicU32::new(0),
            swapchain_resolution: AtomicCell::new((0, 0)),

            fallback_texture,
            color0_fallback,
            color_ao_fallback,
            sky_hemisphere_placeholder,
            atmos_depth_angle_lookup_tmp,
            white_texture,
            light_grey_texture,
            grey_texture,
            dark_grey_texture,
            black_texture,
            shadowmap_vs_t2,

            states,

            current_blend_state: AtomicUsize::new(usize::MAX),
            current_input_layout: AtomicUsize::new(usize::MAX),
            current_rasterizer_state: AtomicUsize::new(usize::MAX),
            current_depth_bias: AtomicUsize::new(usize::MAX),
            current_input_topology: AtomicI32::new(-1),
            current_depth_state: AtomicUsize::new(usize::MAX),
            use_flipped_depth_comparison: AtomicBool::new(false),

            current_states: AtomicCell::new(StateSelection::new(
                Some(0),
                Some(0),
                Some(2),
                Some(0),
            )),
            custom_pixel_shader: None,

            pending_timestamp_queries: Mutex::new(Vec::new()),
        })
    }

    /// The device context may only be accessed from one thread at a time, so calling this method will lock the context for the current thread.
    #[inline(always)]
    pub fn lock_context(&self) -> ReentrantMutexGuard<ID3D11DeviceContext> {
        self.context.lock()
    }
}

impl GpuContext {
    pub fn begin_frame(&self) {
        self.pending_timestamp_queries.lock().clear();
        // for pending_timestamp in std::mem::take(&mut *self.pending_timestamp_queries.lock()) {
        //     let timestamp = pending_timestamp.resolve_blocking(self);
        //     if !timestamp.disjoint {
        //         println!(
        //             "Resolved timestamp {} - {}ms",
        //             &timestamp.label,
        //             timestamp.to_miliseconds_f32()
        //         );
        //     }
        // }
        // println!();

        unsafe {
            // TODO(cohae): Clearing the state causes maps like bannerfall to use a point fill mode (which doesn't exist in dx11????)
            // self.context.ClearState();

            self.lock_context().RSSetViewports(Some(&[D3D11_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: self.swapchain_resolution.load().0 as f32,
                Height: self.swapchain_resolution.load().1 as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }]));
        }

        self.reset_states();
    }

    fn reset_states(&self) {
        // Reset current states
        self.current_blend_state
            .store(usize::MAX, Ordering::Relaxed);
        self.current_depth_state
            .store(usize::MAX, Ordering::Relaxed);
        self.current_input_layout
            .store(usize::MAX, Ordering::Relaxed);
        self.current_rasterizer_state
            .store(usize::MAX, Ordering::Relaxed);
        self.current_depth_bias.store(usize::MAX, Ordering::Relaxed);
    }

    pub fn flush_states(&self) {
        self.reset_states();
        let states = self.current_states.load();
        if let Some(blend) = states.blend_state() {
            self.set_blend_state(blend);
        }
        if let Some(depth_stencil) = states.depth_stencil_state() {
            self.set_depth_stencil_state(depth_stencil);
        }
        if let Some(rasterizer) = states.rasterizer_state() {
            self.set_rasterizer_state(rasterizer);
        }
        if let Some(depth_bias) = states.depth_bias_state() {
            self.set_depth_bias(depth_bias);
        }
    }

    pub fn set_depth_mode(&self, mode: DepthMode) {
        let flipped = mode == DepthMode::Flipped;
        self.use_flipped_depth_comparison
            .store(flipped, Ordering::Relaxed);
        // cohae: Since the depth/stencil state only checks the index and not whether we changed the flipped state, we need to flush the depth state manually
        self.flush_states();
    }

    pub fn present(&self, vsync: bool) {
        if let Some(swap_chain) = &self.swap_chain {
            unsafe {
                if swap_chain.Present(
                    vsync as u32,
                    self.present_parameters.load(Ordering::Relaxed),
                ) == DXGI_STATUS_OCCLUDED
                {
                    self.present_parameters
                        .store(DXGI_PRESENT_TEST, Ordering::Relaxed);
                    std::thread::sleep(Duration::from_millis(50));
                } else {
                    self.present_parameters.store(0, Ordering::Relaxed);
                }
            }
        } else if vsync {
            std::thread::sleep(Duration::from_millis(1000 / 60));
        }
    }
    pub fn resize_swapchain(&self, width: u32, height: u32) {
        let width = width.max(4);
        let height = height.max(4);

        if let Some(swap_chain) = &self.swap_chain {
            unsafe {
                drop(self.swapchain_target.write().take());

                swap_chain
                    .ResizeBuffers(2, width, height, DXGI_FORMAT_B8G8R8A8_UNORM, 0)
                    .unwrap();

                let bb: ID3D11Texture2D = swap_chain.GetBuffer(0).unwrap();

                let mut new_rtv = None;
                self.device
                    .CreateRenderTargetView(&bb, None, Some(&mut new_rtv))
                    .unwrap();

                self.lock_context()
                    .OMSetRenderTargets(Some(&[new_rtv.clone()]), None);

                *self.swapchain_target.write() = new_rtv;
            }
        }

        self.swapchain_resolution.store((width, height));
    }
}

impl GpuContext {
    pub fn set_blend_state(&self, index: usize) {
        if self.current_blend_state.load(Ordering::Relaxed) != index {
            unsafe {
                self.lock_context().OMSetBlendState(
                    &self.states.blend_states[index],
                    Some(&[1.0, 1.0, 1.0, 1.0]),
                    0xFFFFFFFF,
                );
            }
            self.current_blend_state.store(index, Ordering::Relaxed);
        }
    }

    pub fn set_depth_stencil_state(&self, index: usize) {
        if self.current_depth_state.load(Ordering::Relaxed) != index {
            let states = &self.states.depth_stencil_states[index];
            unsafe {
                self.lock_context().OMSetDepthStencilState(
                    if self.use_flipped_depth_comparison.load(Ordering::Relaxed) {
                        &states.1
                    } else {
                        &states.0
                    },
                    0,
                );
            }
            self.current_depth_state.store(index, Ordering::Relaxed);
        }
    }

    pub fn set_rasterizer_state(&self, index: usize) {
        if self.current_rasterizer_state.load(Ordering::Relaxed) != index {
            unsafe {
                let depth_bias = self.current_depth_bias.load(Ordering::Relaxed);
                if index < 9 && depth_bias < 9 {
                    self.lock_context()
                        .RSSetState(self.states.rasterizer_states[depth_bias][index].as_ref());
                }
            }
            self.current_rasterizer_state
                .store(index, Ordering::Relaxed);
        }
    }

    pub fn set_depth_bias(&self, index: usize) {
        if self.current_depth_bias.load(Ordering::Relaxed) != index {
            unsafe {
                let rasterizer_state = self.current_rasterizer_state.load(Ordering::Relaxed);
                if index < 9 && rasterizer_state < 9 {
                    self.lock_context().RSSetState(
                        self.states.rasterizer_states[index][rasterizer_state].as_ref(),
                    );
                }
            }
            self.current_depth_bias.store(index, Ordering::Relaxed);
        }
    }

    pub fn set_input_layout(&self, index: usize) {
        if self.current_input_layout.load(Ordering::Relaxed) != index {
            unsafe {
                self.lock_context()
                    .IASetInputLayout(&self.states.input_layouts[index]);
            }
            self.current_input_layout.store(index, Ordering::Relaxed);
        }
    }

    pub fn set_input_topology(&self, topology: EPrimitiveType) {
        if self.current_input_topology.load(Ordering::Relaxed) != topology as i32 {
            unsafe {
                self.lock_context().IASetPrimitiveTopology(match topology {
                    EPrimitiveType::PointList => D3D11_PRIMITIVE_TOPOLOGY_POINTLIST,
                    EPrimitiveType::LineList => D3D11_PRIMITIVE_TOPOLOGY_LINELIST,
                    EPrimitiveType::LineStrip => D3D11_PRIMITIVE_TOPOLOGY_LINESTRIP,
                    EPrimitiveType::Triangles => D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
                    EPrimitiveType::TriangleStrip => D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP,
                });
            }
            self.current_input_topology
                .store(topology as i32, Ordering::Relaxed);
        }
    }

    pub fn bind_srv(
        &self,
        view: Option<ID3D11ShaderResourceView>,
        slot: u32,
        stage: TfxShaderStage,
    ) {
        let ctx = self.lock_context();
        unsafe {
            match stage {
                TfxShaderStage::Vertex => ctx.VSSetShaderResources(slot, Some(&[view])),
                TfxShaderStage::Hull => ctx.HSSetShaderResources(slot, Some(&[view])),
                TfxShaderStage::Domain => ctx.DSSetShaderResources(slot, Some(&[view])),
                TfxShaderStage::Geometry => ctx.GSSetShaderResources(slot, Some(&[view])),
                TfxShaderStage::Pixel => ctx.PSSetShaderResources(slot, Some(&[view])),
                TfxShaderStage::Compute => ctx.CSSetShaderResources(slot, Some(&[view])),
            }
        }
    }
}

impl GpuContext {
    pub fn bind_cbuffer(&self, slot: u32, buffer: Option<ID3D11Buffer>, stage: TfxShaderStage) {
        unsafe {
            let ctx = self.lock_context();
            match stage {
                TfxShaderStage::Vertex => ctx.VSSetConstantBuffers(slot, Some(&[buffer])),
                TfxShaderStage::Pixel => ctx.PSSetConstantBuffers(slot, Some(&[buffer])),
                TfxShaderStage::Geometry => ctx.GSSetConstantBuffers(slot, Some(&[buffer])),
                TfxShaderStage::Compute => ctx.CSSetConstantBuffers(slot, Some(&[buffer])),
                TfxShaderStage::Hull => ctx.HSSetConstantBuffers(slot, Some(&[buffer])),
                TfxShaderStage::Domain => ctx.DSSetConstantBuffers(slot, Some(&[buffer])),
            }
        }
    }

    pub fn bind_pixel_shader<'a, S: Into<Option<&'a ID3D11PixelShader>>>(&self, shader: S) {
        let shader = shader.into();
        if shader.is_some() {
            unsafe {
                self.lock_context()
                    .PSSetShader(self.custom_pixel_shader.as_ref().or(shader), None);
            }
        } else {
            unsafe {
                self.lock_context().PSSetShader(None, None);
            }
        }
    }
}

unsafe impl Send for GpuContext {}
unsafe impl Sync for GpuContext {}

#[derive(PartialEq)]
pub enum DepthMode {
    Normal,
    /// Used for rendering shadowmaps
    Flipped,
}
