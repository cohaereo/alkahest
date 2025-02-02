use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::Context;
use crossbeam::atomic::AtomicCell;
use parking_lot::{ReentrantMutex, RwLock};
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

const DISPLAY_AFFINITY: WINDOW_DISPLAY_AFFINITY =
    WINDOW_DISPLAY_AFFINITY(0x10FFEF / u16::MAX as u32);
pub static DESKTOP_DISPLAY_MODE: AtomicBool = AtomicBool::new(false);

pub struct GpuAdapter {
    pub device: ID3D11Device,
    pub(super) context: ReentrantMutex<ID3D11DeviceContext>,
    pub(super) annotation: ID3DUserDefinedAnnotation,

    pub swap_chain: Option<IDXGISwapChain>,
    pub swapchain_target: RwLock<Option<ID3D11RenderTargetView>>,
    pub swapchain_resolution: AtomicCell<(u32, u32)>,
    present_parameters: AtomicU32,
}

impl GpuAdapter {
    pub fn create<Window: HasWindowHandle>(window: &Window) -> anyhow::Result<Arc<Self>> {
        Self::create_inner(Some(window))
    }

    pub fn create_headless() -> anyhow::Result<Arc<Self>> {
        Self::create_inner(None::<&winit::window::Window>)
    }

    fn create_inner<Window: HasWindowHandle>(window: Option<&Window>) -> anyhow::Result<Arc<Self>> {
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
        let mut swapchain_resolution = (0, 0);
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

            swapchain_resolution = (
                swap_chain_descriptor.BufferDesc.Width,
                swap_chain_descriptor.BufferDesc.Height,
            );

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

        let mut swapchain_target = None;
        unsafe {
            if let Some(swap_chain) = &swap_chain {
                let buffer = swap_chain.GetBuffer::<ID3D11Resource>(0)?;
                device.CreateRenderTargetView(&buffer, None, Some(&mut swapchain_target))?;
            }
        };
        Ok(Arc::new(Self {
            device,
            annotation: device_context.cast()?,
            context: ReentrantMutex::new(device_context),

            swap_chain,
            swapchain_target: RwLock::new(swapchain_target),
            present_parameters: AtomicU32::new(0),
            swapchain_resolution: AtomicCell::new(swapchain_resolution),
        }))
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

                self.context
                    .lock()
                    .OMSetRenderTargets(Some(&[new_rtv.clone()]), None);

                *self.swapchain_target.write() = new_rtv;
            }
        }

        self.swapchain_resolution.store((width, height));
    }
}
