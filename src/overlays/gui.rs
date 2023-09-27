use std::cell::RefCell;
use std::mem::transmute;
use std::rc::Rc;
use std::sync::Arc;

use crate::render::DeviceContextSwapchain;
use crate::resources::Resources;
use egui_winit::EventResponse;
use winit::event::WindowEvent;
use winit::window::Window;

//TODO: Pass GUI Manager to get other overlays
pub trait OverlayProvider {
    fn draw(&mut self, ctx: &egui::Context, window: &Window, resources: &mut Resources);
}
pub struct GuiManager {
    pub egui: egui::Context,
    pub integration: egui_winit::State,
    pub renderer: egui_directx11::DirectX11Renderer,
    overlays: Vec<Rc<RefCell<dyn OverlayProvider>>>,
    dcs: Arc<DeviceContextSwapchain>,
}

// TODO: Way to obtain overlays by type
impl GuiManager {
    pub fn create(window: &Window, dcs: Arc<DeviceContextSwapchain>) -> Self {
        let egui = egui::Context::default();
        // imgui.style_mut().window_rounding = 4.0;
        let mut integration = egui_winit::State::new(window);
        integration.set_pixels_per_point(window.scale_factor() as f32);

        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "materialdesignicons".into(),
            egui::FontData::from_static(include_bytes!("../../materialdesignicons-webfont.ttf")),
        );

        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(1, "materialdesignicons".to_owned());

        egui.set_fonts(fonts);

        let renderer = egui_directx11::DirectX11Renderer::init_from_swapchain(unsafe {
            transmute(&dcs.swap_chain)
        })
        .expect("Failed to initialize egui renderer");

        GuiManager {
            egui,
            integration,
            renderer,
            overlays: vec![],
            dcs,
        }
    }

    pub fn add_overlay(&mut self, overlay: Rc<RefCell<dyn OverlayProvider>>) {
        self.overlays.push(overlay);
    }

    pub fn handle_event(&mut self, event: &WindowEvent<'_>) -> EventResponse {
        self.integration.on_event(&self.egui, event)
    }

    pub fn draw_frame<MF>(&mut self, window: Arc<Window>, resources: &mut Resources, misc_draw: MF)
    where
        MF: FnOnce(&egui::Context),
    {
        let input = self.integration.take_egui_input(&window);

        self.renderer
            .paint(
                unsafe { transmute(&self.dcs.swap_chain) },
                input,
                &self.egui,
                |ctx| {
                    for overlay in self.overlays.iter() {
                        overlay.as_ref().borrow_mut().draw(ctx, &window, resources);
                    }

                    misc_draw(ctx);
                },
            )
            .unwrap();
    }
}
