use std::cell::RefCell;
use std::mem::transmute;
use std::rc::Rc;
use std::sync::Arc;

use crate::render::DeviceContextSwapchain;
use crate::resources::Resources;
use crate::util::exe_relative_path;
use crate::util::image::Png;
use egui_winit::EventResponse;
use winit::event::WindowEvent;
use winit::window::Window;

//TODO: Pass GUI Manager to get other overlays
pub trait OverlayProvider {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        window: &Window,
        resources: &mut Resources,
        icons: &GuiResources,
    );
}
pub struct GuiManager {
    pub egui: egui::Context,
    pub integration: egui_winit::State,
    pub renderer: egui_directx11::DirectX11Renderer,
    overlays: Vec<Rc<RefCell<dyn OverlayProvider>>>,
    dcs: Arc<DeviceContextSwapchain>,
    resources: GuiResources,
}

// TODO: Way to obtain overlays by type
impl GuiManager {
    pub fn create(window: &Window, dcs: Arc<DeviceContextSwapchain>) -> Self {
        let egui = egui::Context::default();

        if let Ok(Ok(data)) = std::fs::read_to_string(exe_relative_path("egui.ron"))
            .map(|s| ron::from_str::<egui::Memory>(&s))
        {
            info!("Loaded egui state from egui.ron");
            egui.memory_mut(|memory| *memory = data);
        }

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
            resources: GuiResources::load(&egui),
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

        let output = self
            .renderer
            .paint(
                unsafe { transmute(&self.dcs.swap_chain) },
                input,
                &self.egui,
                |ctx| {
                    for overlay in self.overlays.iter() {
                        overlay.as_ref().borrow_mut().draw(
                            ctx,
                            &window,
                            resources,
                            &self.resources,
                        );
                    }

                    misc_draw(ctx);
                },
            )
            .unwrap();

        self.integration
            .handle_platform_output(&window, &self.egui, output.platform_output)
    }
}

impl Drop for GuiManager {
    fn drop(&mut self) {
        match self.egui.memory(ron::to_string) {
            Ok(memory) => {
                if let Err(e) = std::fs::write(exe_relative_path("egui.ron"), memory) {
                    error!("Failed to write egui state: {e}");
                }
            }
            Err(e) => {
                error!("Failed to serialize egui state: {e}");
            }
        };
    }
}

pub struct GuiResources {
    pub icon_havok: egui::TextureHandle,
}

impl GuiResources {
    pub fn load(ctx: &egui::Context) -> Self {
        let img = Png::from_bytes(include_bytes!("../../assets/icons/havok_dark_256.png")).unwrap();
        let icon_havok = ctx.load_texture(
            "Havok 64x64",
            egui::ImageData::Color(egui::ColorImage::from_rgba_premultiplied(
                img.dimensions,
                &img.data,
            )),
            egui::TextureOptions {
                magnification: egui::TextureFilter::Linear,
                minification: egui::TextureFilter::Linear,
            },
        );

        Self { icon_havok }
    }
}
