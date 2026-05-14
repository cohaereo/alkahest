use std::time::Duration;

use d3d11::dxgi::{self, PresentFlags, SwapChainFlags, SwapChainStatus};

pub struct Swapchain {
    pub swapchain: d3d11::dxgi::SwapChain,
    pub swapchain_target: Option<d3d11::RenderTargetView>,
    pub(crate) swapchain_resolution: (u32, u32),
    present_parameters: PresentFlags,
}

impl Swapchain {
    pub fn new(
        swapchain: d3d11::dxgi::SwapChain,
        device: &d3d11::Device,
        size: (u32, u32),
    ) -> Self {
        let mut s = Self {
            swapchain,
            swapchain_target: None,
            swapchain_resolution: size,
            present_parameters: PresentFlags::empty(),
        };

        s.resize(device, size);

        s
    }

    pub fn get_buffer(&self) -> d3d11::Texture2D {
        self.swapchain.get_buffer(0).unwrap()
    }

    // ⚠ The calling function MUST ensure that the RTV is not held/in use.
    pub fn resize(&mut self, device: &d3d11::Device, new_size: (u32, u32)) {
        drop(self.swapchain_target.take());

        self.swapchain
            .resize_buffers(
                2,
                new_size.0,
                new_size.1,
                dxgi::Format::B8g8r8a8Unorm,
                SwapChainFlags::empty(),
            )
            .unwrap();

        let bb: d3d11::Texture2D = self.swapchain.get_buffer(0).unwrap();

        let new_rtv = device.create_render_target_view(&bb, None).unwrap();

        self.swapchain_target = Some(new_rtv);
        self.swapchain_resolution = new_size;
    }

    pub(crate) fn present(&mut self, vsync: bool) {
        if self
            .swapchain
            .present(vsync as u32, self.present_parameters)
            == Some(SwapChainStatus::Occluded)
        {
            self.present_parameters = PresentFlags::TEST;
            std::thread::sleep(Duration::from_millis(50));
        } else {
            self.present_parameters = PresentFlags::empty();
        }
    }
}
