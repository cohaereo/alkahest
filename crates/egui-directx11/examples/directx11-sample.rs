use egui_directx11::DirectX11Renderer;
use std::mem::transmute;
use windows::core::ComInterface;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::platform::windows::*;
use winit::window::WindowBuilder;

const WINDOW_WIDTH: f64 = 760.0;
const WINDOW_HEIGHT: f64 = 760.0;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn create_device_with_type(drive_type: D3D_DRIVER_TYPE) -> Result<ID3D11Device> {
    let mut flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;

    if cfg!(debug_assertions) {
        flags |= D3D11_CREATE_DEVICE_DEBUG;
    }

    let mut device = None;
    let feature_levels = [D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_10_0];
    let mut fl = D3D_FEATURE_LEVEL_11_1;
    unsafe {
        Ok(D3D11CreateDevice(
            None,
            drive_type,
            HMODULE::default(),
            flags,
            Some(&feature_levels),
            D3D11_SDK_VERSION,
            Some(&mut device),
            Some(&mut fl),
            None,
        )
        .map(|()| device.unwrap())?)
    }
}

fn create_device() -> Result<ID3D11Device> {
    create_device_with_type(D3D_DRIVER_TYPE_HARDWARE)
}

fn create_swapchain(device: &ID3D11Device, window: HWND) -> Result<IDXGISwapChain> {
    let factory = get_dxgi_factory(device)?;

    let sc_desc = DXGI_SWAP_CHAIN_DESC {
        BufferDesc: DXGI_MODE_DESC {
            Width: 0,
            Height: 0,
            RefreshRate: DXGI_RATIONAL {
                Numerator: 60,
                Denominator: 1,
            },
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            ..Default::default()
        },
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 3,
        OutputWindow: window,
        Windowed: true.into(),
        SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
        Flags: DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH.0 as u32,
    };

    let mut swapchain = None;
    let _hresult = unsafe { factory.CreateSwapChain(device, &sc_desc, &mut swapchain) };
    Ok(swapchain.unwrap())
}

fn get_dxgi_factory(device: &ID3D11Device) -> Result<IDXGIFactory2> {
    let dxdevice = device.cast::<IDXGIDevice>()?;
    unsafe { Ok(dxdevice.GetAdapter()?.GetParent()?) }
}

fn main() -> Result<()> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("egui-directx11")
        .with_inner_size(LogicalSize {
            width: WINDOW_WIDTH,
            height: WINDOW_HEIGHT,
        })
        .build(&event_loop)
        .unwrap();

    let device = create_device()?;
    let swapchain = unsafe { create_swapchain(&device, transmute(window.hwnd()))? };

    struct State {
        counter: u32,
    }
    let mut shared_state = State { counter: 5 };
    let mut dx_renderer: DirectX11Renderer =
        DirectX11Renderer::init_from_swapchain(&swapchain, egui::Context::default())?;

    event_loop.run(move |event, _, control_flow| match event {
        Event::MainEventsCleared => {
            window.request_redraw();
        }

        Event::RedrawRequested(_) => {
            // collect input here
            let input = egui::RawInput::default();
            dx_renderer
                .paint(&swapchain, &mut shared_state, input, |ctx, state| {
                    egui::Window::new("Test Window").show(ctx, |ui| {
                        ui.label("Hi");
                        ui.add(egui::Slider::new(&mut state.counter, 0..=120).text("test state"));
                    });
                })
                .expect("successful render");

            // you handle the swapchain present
            let _ = unsafe { swapchain.Present(1, 0) };
        }
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = winit::event_loop::ControlFlow::Exit,
        Event::WindowEvent {
            event:
                WindowEvent::Resized(winit::dpi::PhysicalSize {
                    height: _height,
                    width: _width,
                }),
            ..
        } => (),
        Event::LoopDestroyed => (),
        _event => (),
    })
}
