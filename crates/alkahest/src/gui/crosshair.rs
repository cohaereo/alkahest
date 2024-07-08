use alkahest_renderer::{resources::Resources};
use egui::{Color32, Context, Stroke};
use winit::window::Window;

use crate::{
    config, gui::context::{GuiCtx, GuiView, ViewResult}, maplist::{MapList, MapLoadState}
};

pub struct CrosshairOverlay;

impl GuiView for CrosshairOverlay {
    fn draw(
        &mut self,
        ctx: &Context,
        _window: &Window,
        resources: &Resources,
        _gui: &GuiCtx<'_>,
    ) -> Option<ViewResult> {
        let maps = resources.get::<MapList>();
        if maps.current_map().map_or(true, |m|m.load_state != MapLoadState::Loaded) {
            return None
        }
        if config::with(|c|!c.visual.draw_crosshair) {
            return None;
        }

        let painter = ctx.layer_painter(egui::LayerId::background());


        let center = ctx.screen_rect().center();
        let width = 2.0;
        let size = 8.0;
        let stroke = Stroke {
            width,
            color: Color32::WHITE,
        };

        painter.line_segment([center + (size, 0.0).into(), center + (-size, 0.0).into()], stroke);
        painter.line_segment([center + (0.0, size).into(), center + (0.0, -size).into()], stroke);
        None
    }
}
