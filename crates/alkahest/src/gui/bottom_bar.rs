use std::time::Instant;

use egui::{Color32, Context, RichText};
use once_cell::sync::Lazy;
use winit::window::Window;

use crate::{
    gui::{
        context::{GuiCtx, GuiView, ViewResult},
        icons,
        icons::{ICON_ALERT_CIRCLE_OUTLINE, ICON_CHECK_CIRCLE, ICON_CIRCLE, ICON_CIRCLE_OUTLINE},
    },
    maplist::{MapList, MapLoadState},
    resources::Resources,
};

pub struct BottomBar;

impl GuiView for BottomBar {
    fn draw(
        &mut self,
        ctx: &Context,
        _window: &Window,
        resources: &Resources,
        _gui: &GuiCtx<'_>,
    ) -> Option<ViewResult> {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            let mut maplist = resources.get_mut::<MapList>();
            if !maplist.maps.is_empty() {
                let mut current_map = maplist.current_map;

                let amount_loaded = maplist.count_loaded();
                let combo_label = if maplist.count_loading() != 0 {
                    format!("Map (loading {}/{})", amount_loaded + 1, maplist.maps.len())
                } else {
                    "Map".to_string()
                };
                ui.horizontal(|ui| {
                    let map_changed = egui::ComboBox::from_label(combo_label)
                        .width(192.0)
                        .show_index(ui, &mut current_map, maplist.maps.len(), |i| {
                            let map = &maplist.maps[i];
                            let (mut icon, color) = match map.load_state {
                                MapLoadState::Unloaded => (ICON_CIRCLE_OUTLINE, Color32::GRAY),
                                MapLoadState::Loading => {
                                    (LoadingIcon::Circle.get_frame(), Color32::WHITE)
                                }
                                MapLoadState::Loaded => (ICON_CIRCLE, Color32::WHITE),
                                MapLoadState::Error(_) => (ICON_ALERT_CIRCLE_OUTLINE, Color32::RED),
                            };

                            if maplist.current_map == i && map.load_state == MapLoadState::Loaded {
                                icon = ICON_CHECK_CIRCLE;
                            }

                            RichText::new(format!("{icon} {}", maplist.maps[i].name)).color(color)
                        })
                        .changed();
                    ui.checkbox(&mut maplist.load_all_maps, "Load all maps");

                    if map_changed {
                        maplist.current_map = current_map;
                    }
                });
            }
        });

        None
    }
}

pub enum LoadingIcon {
    /// A simple, indeterminate spinning clock
    Clock,
    // Indeterminate circle slice animation
    Circle,
}

pub static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

impl LoadingIcon {
    pub const CLOCK_FRAMES: [char; 12] = [
        icons::ICON_CLOCK_TIME_ONE_OUTLINE,
        icons::ICON_CLOCK_TIME_TWO_OUTLINE,
        icons::ICON_CLOCK_TIME_THREE_OUTLINE,
        icons::ICON_CLOCK_TIME_FOUR_OUTLINE,
        icons::ICON_CLOCK_TIME_FIVE_OUTLINE,
        icons::ICON_CLOCK_TIME_SIX_OUTLINE,
        icons::ICON_CLOCK_TIME_SEVEN_OUTLINE,
        icons::ICON_CLOCK_TIME_EIGHT_OUTLINE,
        icons::ICON_CLOCK_TIME_NINE_OUTLINE,
        icons::ICON_CLOCK_TIME_TEN_OUTLINE,
        icons::ICON_CLOCK_TIME_ELEVEN_OUTLINE,
        icons::ICON_CLOCK_TIME_TWELVE_OUTLINE,
    ];

    pub const CIRCLE_FRAMES: [char; 8] = [
        icons::ICON_CIRCLE_SLICE_1,
        icons::ICON_CIRCLE_SLICE_2,
        icons::ICON_CIRCLE_SLICE_3,
        icons::ICON_CIRCLE_SLICE_4,
        icons::ICON_CIRCLE_SLICE_5,
        icons::ICON_CIRCLE_SLICE_6,
        icons::ICON_CIRCLE_SLICE_7,
        icons::ICON_CIRCLE_SLICE_8,
    ];

    pub const CLOCK_INTERVAL: usize = 50;
    pub const CIRCLE_INTERVAL: usize = 100;

    pub fn get_frame(&self) -> char {
        self.get_frame_with_time(*START_TIME)
    }

    pub fn get_frame_with_time(&self, time: Instant) -> char {
        let time_millis = time.elapsed().as_millis() as usize;
        match self {
            LoadingIcon::Clock => {
                Self::CLOCK_FRAMES[(time_millis / Self::CLOCK_INTERVAL) % Self::CLOCK_FRAMES.len()]
            }
            LoadingIcon::Circle => {
                Self::CIRCLE_FRAMES
                    [(time_millis / Self::CIRCLE_INTERVAL) % Self::CIRCLE_FRAMES.len()]
            }
        }
    }
}
