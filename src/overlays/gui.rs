use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::icons::{ICON_MAX, ICON_MIN};
use crate::resources::Resources;
use imgui::{Context, FontConfig, FontGlyphRanges, FontSource};
use imgui_dx11_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use windows::Win32::Graphics::Direct3D11::ID3D11Device;
use winit::event::Event;
use winit::window::Window;

//TODO: Pass GUI Manager to get other overlays
pub trait OverlayProvider {
    fn create_overlay(&mut self, ui: &mut imgui::Ui, window: &Window, resources: &mut Resources);
}

pub struct GuiManager {
    pub imgui: Context,
    pub platform: WinitPlatform,
    renderer: Renderer,
    overlays: Vec<Rc<RefCell<dyn OverlayProvider>>>,
}

// TODO: Way to obtain overlays by type
impl GuiManager {
    pub fn create(window: &Window, device: &ID3D11Device) -> Self {
        let mut imgui = imgui::Context::create();
        imgui.style_mut().window_rounding = 4.0;
        let mut platform = WinitPlatform::init(&mut imgui);
        platform.attach_window(imgui.io_mut(), window, HiDpiMode::Rounded);

        // Combine icon font with default
        imgui.fonts().add_font(&[
            FontSource::DefaultFontData {
                config: Some(FontConfig {
                    size_pixels: (13.0 * platform.hidpi_factor()) as f32,
                    glyph_ranges: FontGlyphRanges::default(),
                    ..FontConfig::default()
                }),
            },
            FontSource::TtfData {
                data: include_bytes!("../../materialdesignicons-webfont.ttf"),
                size_pixels: (13.0 * platform.hidpi_factor()) as f32,
                config: Some(FontConfig {
                    size_pixels: (13.0 * platform.hidpi_factor()) as f32,
                    glyph_ranges: FontGlyphRanges::from_slice(&[
                        ICON_MIN as u32,
                        ICON_MAX as u32,
                        0,
                    ]),
                    ..FontConfig::default()
                }),
            },
        ]);

        let renderer = unsafe { imgui_dx11_renderer::Renderer::new(&mut imgui, device).unwrap() };

        GuiManager {
            imgui,
            platform,
            renderer,
            overlays: vec![],
        }
    }

    pub fn add_overlay(&mut self, overlay: Rc<RefCell<dyn OverlayProvider>>) {
        self.overlays.push(overlay);
    }

    pub fn handle_event(&mut self, event: &Event<'_, ()>, window: &Window) {
        self.platform
            .handle_event(self.imgui.io_mut(), window, event)
    }

    pub fn draw_frame(&mut self, window: &Window, delta: Duration, resources: &mut Resources) {
        self.imgui.io_mut().update_delta_time(delta);
        let ui = self.imgui.new_frame();

        for overlay in self.overlays.iter() {
            overlay
                .as_ref()
                .borrow_mut()
                .create_overlay(ui, window, resources);
        }

        self.platform.prepare_render(ui, window);
        self.renderer
            .render(self.imgui.render())
            .expect("GuiManager failed to render!");
    }
}
