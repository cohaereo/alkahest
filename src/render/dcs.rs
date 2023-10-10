use crate::util::RwLock;
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use std::mem::transmute;
use std::thread::ThreadId;
use windows::Win32::Foundation::{BOOL, HINSTANCE};
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use winit::window::Window;

pub struct DeviceContextSwapchain {
    main_thread_id: ThreadId,

    pub device: ID3D11Device,
    context: ID3D11DeviceContext,
    pub swap_chain: IDXGISwapChain,
    pub swapchain_target: RwLock<Option<ID3D11RenderTargetView>>,
}

impl DeviceContextSwapchain {
    #[allow(const_item_mutation)]
    pub fn create(window: &Window) -> anyhow::Result<Self> {
        let mut device: Option<ID3D11Device> = None;
        let mut swap_chain: Option<IDXGISwapChain> = None;
        let mut device_context: Option<ID3D11DeviceContext> = None;
        let swap_chain_description: DXGI_SWAP_CHAIN_DESC = {
            let buffer_descriptor = {
                let refresh_rate = DXGI_RATIONAL {
                    Numerator: 0,
                    Denominator: 0,
                };

                DXGI_MODE_DESC {
                    Width: 0,
                    Height: 0,
                    RefreshRate: refresh_rate,
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM_SRGB,
                    ScanlineOrdering: DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED,
                    Scaling: DXGI_MODE_SCALING_UNSPECIFIED,
                }
            };

            let sample_descriptor = DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            };

            DXGI_SWAP_CHAIN_DESC {
                BufferDesc: buffer_descriptor,
                SampleDesc: sample_descriptor,
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT | DXGI_USAGE_SHADER_INPUT,
                BufferCount: 1,
                OutputWindow: match window.raw_window_handle() {
                    RawWindowHandle::Win32(h) => unsafe { transmute(h.hwnd) },
                    u => panic!("Can't open window for {u:?}"),
                },
                Windowed: BOOL(1),
                SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
                Flags: 0,
            }
        };

        unsafe {
            D3D11CreateDeviceAndSwapChain(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HINSTANCE::default(),
                Default::default(), // | D3D11_CREATE_DEVICE_DEBUG,
                Some(&[D3D_FEATURE_LEVEL_11_1]),
                D3D11_SDK_VERSION,
                Some(&swap_chain_description),
                Some(&mut swap_chain),
                Some(&mut device),
                Some(&mut D3D_FEATURE_LEVEL_11_1),
                Some(&mut device_context),
            )?;
        }

        let device = device.unwrap();
        let device_context = device_context.unwrap();
        let swap_chain = swap_chain.unwrap();

        let swapchain_target = unsafe {
            let buffer = swap_chain.GetBuffer::<ID3D11Resource>(0)?;
            Some(device.CreateRenderTargetView(&buffer, None)?)
        };

        Ok(Self {
            main_thread_id: std::thread::current().id(),
            device,
            context: device_context,
            swap_chain,
            swapchain_target: RwLock::new(swapchain_target),
        })
    }

    /// The device context may only be accessed from the thread that the DCS was created on
    /// Panics if the current thread is not the main thread
    pub fn context(&self) -> &ID3D11DeviceContext {
        assert_eq!(std::thread::current().id(), self.main_thread_id, "Tried to access ID3D11DeviceContext from thread {:?}, but context was created on thread {:?}", std::thread::current().id(), self.main_thread_id);

        &self.context
    }
}

unsafe impl Send for DeviceContextSwapchain {}
unsafe impl Sync for DeviceContextSwapchain {}
