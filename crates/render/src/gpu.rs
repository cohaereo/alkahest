pub mod cbuffer;
pub mod command_list;
pub mod debug_text;
mod global_state;
pub mod profiler;
pub mod spinner;
pub mod state;
pub mod swapchain;

use std::{rc::Rc, sync::Arc};

use anyhow::Context;
use d3d11::{
    dxgi::{self, DxgiUsage, ModeDesc, SwapChainDesc},
    sys::{
        Dxgi::{
            CreateDXGIFactory, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, DXGI_QUERY_VIDEO_MEMORY_INFO,
            IDXGIAdapter3, IDXGIFactory,
        },
        core::Interface,
    },
};
use parking_lot::{Mutex, ReentrantMutex, ReentrantMutexGuard};
use swapchain::Swapchain;

use crate::asset::texture::Texture;

pub struct Gpu {
    pub placeholder_white: Texture,
    pub placeholder_ao: Texture,
    pub placeholder_normal: Texture,
    adapter: IDXGIAdapter3,
    pub device: d3d11::Device,
    context: ReentrantMutex<d3d11::DeviceContext>,
    // TODO(cohae): Should this be moved to the Renderer?
    pub swapchain: Mutex<Swapchain>,

    global_states: global_state::RenderStates,
}

unsafe impl Sync for Gpu {}
unsafe impl Send for Gpu {}

#[profiling::all_functions]
impl Gpu {
    pub fn create(window: &Rc<sdl3::video::Window>) -> anyhow::Result<Self> {
        let dxgi: IDXGIFactory = unsafe { CreateDXGIFactory() }?;

        let adapter = unsafe { dxgi.EnumAdapters(0).context("No adapters found")? };
        let adapter3 = adapter
            .cast::<IDXGIAdapter3>()
            .context("Couldn't find a compatible adapter")?;

        #[cfg(target_os = "windows")]
        let output_window = {
            use d3d11::HWND;
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            match window.window_handle().unwrap().as_raw() {
                RawWindowHandle::Win32(h) => HWND(h.hwnd.get() as *mut _),
                u => panic!("Can't open window for {u:?}"),
            }
        };

        #[cfg(not(target_os = "windows"))]
        let output_window = window;

        let device =
            d3d11::Device::create(Some(adapter), false).context("Failed to create device")?;
        let context = device.get_immediate_context();

        let swap_chain = d3d11::dxgi::SwapChain::create(
            &device,
            output_window,
            &SwapChainDesc::builder()
                .buffer_desc(
                    ModeDesc::builder()
                        .format(dxgi::Format::R8g8b8a8Unorm)
                        .build(),
                )
                .buffer_usage(DxgiUsage::RENDER_TARGET_OUTPUT)
                .buffer_count(2)
                .swap_effect(dxgi::SwapEffect::FlipDiscard)
                .build(),
        )
        .context("Failed to create swap chain")?;

        let window_size = window.size();
        Ok(Self {
            placeholder_white: Texture::load_2d_raw(
                &device,
                2,
                2,
                &[0xff; 4 * 2 * 2],
                dxgi::Format::R8g8b8a8Unorm,
                None,
                false,
            )?,
            placeholder_ao: Texture::load_2d_raw(
                &device,
                4096,
                4096,
                &[0xff; 4096 * 4096],
                dxgi::Format::R8Unorm,
                None,
                false,
            )?,
            #[rustfmt::skip]
            placeholder_normal: Texture::load_2d_raw(
                &device,
                2,
                2,
                &[
                    0x7f, 0x7f, 0xff, 0xff, 0x7f, 0x7f, 0xff, 0xff,
                    0x7f, 0x7f, 0xff, 0xff, 0x7f, 0x7f, 0xff, 0xff,
                ],
                dxgi::Format::R8g8b8a8Unorm,
                None,
                false,
            )?,
            adapter: adapter3,
            swapchain: Mutex::new(Swapchain::new(swap_chain, &device, window_size)),
            global_states: global_state::RenderStates::new(&device)
                .context("Failed to create global render states")?,
            device,
            context: ReentrantMutex::new(context),
        })
    }

    #[inline(always)]
    pub fn context(&'_ self) -> ReentrantMutexGuard<'_, d3d11::DeviceContext> {
        self.context.lock()
    }

    pub fn create_command_list(self: &Arc<Self>) -> command_list::CommandList {
        command_list::CommandList::new(self)
    }

    pub fn submit_command_list(&self, cmd: command_list::CommandList) {
        self.context().execute_command_list(
            &cmd.finish_command_list(false)
                .expect("Failed to finish command list"),
            false,
        );
    }
}

// Swapchain management
impl Gpu {
    #[profiling::function]
    pub fn present(&self, vsync: bool) {
        self.swapchain.lock().present(vsync);
    }

    pub fn acquire_rtv(&self) -> d3d11::RenderTargetView {
        self.swapchain
            .lock()
            .swapchain_target
            .as_ref()
            .unwrap()
            .clone()
    }

    pub fn swapchain_resolution(&self) -> (u32, u32) {
        self.swapchain.lock().swapchain_resolution
    }

    #[profiling::function]
    pub fn resize_swapchain(&self, size: (u32, u32)) {
        self.swapchain.lock().resize(&self.device, size);
    }
}

// Adapter info
impl Gpu {
    pub fn get_adapter_name(&self) -> String {
        unsafe {
            let d = self.adapter.GetDesc().unwrap().Description;
            let len = d.iter().position(|&x| x == 0).unwrap();
            String::from_utf16_lossy(&d[..len]).to_string()
        }
    }

    pub fn get_memory_stats(&self) -> DXGI_QUERY_VIDEO_MEMORY_INFO {
        unsafe {
            let mut memory_info = DXGI_QUERY_VIDEO_MEMORY_INFO::default();
            self.adapter
                .QueryVideoMemoryInfo(0, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, &mut memory_info)
                .unwrap();

            memory_info
        }
    }
}

// Misc helpers
#[profiling::all_functions]
impl Gpu {
    pub fn compile_shader_vs_ps(
        &self,
        name: &str,
        source: &str,
        entry_vs: &str,
        entry_ps: &str,
    ) -> anyhow::Result<(d3d11::VertexShader, d3d11::PixelShader)> {
        let vs_source = d3d11::fxc::compile(
            source.as_bytes(),
            Some(name),
            &[],
            entry_vs,
            d3d11::fxc::ShaderTarget::Vertex,
        )?;
        let ps_source = d3d11::fxc::compile(
            source.as_bytes(),
            Some(name),
            &[],
            entry_ps,
            d3d11::fxc::ShaderTarget::Pixel,
        )?;

        let vs = self
            .create_vertex_shader(&vs_source)
            .context("Failed to create vertex shader")?;
        let ps = self
            .create_pixel_shader(&ps_source)
            .context("Failed to create pixel shader")?;

        Ok((vs, ps))
    }

    pub fn compile_shader_cs(
        &self,
        name: &str,
        source: &str,
        entry_cs: &str,
    ) -> anyhow::Result<d3d11::ComputeShader> {
        let cs_source = d3d11::fxc::compile(
            source.as_bytes(),
            Some(name),
            &[],
            entry_cs,
            d3d11::fxc::ShaderTarget::Compute,
        )?;

        let cs = self
            .create_compute_shader(&cs_source)
            .context("Failed to create compute shader")?;

        Ok(cs)
    }
}

impl std::ops::Deref for Gpu {
    type Target = d3d11::Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}
