use std::{
    sync::{Arc, LazyLock},
    time::Instant,
};

use egui::{Color32, FontId, RichText, TextStyle, Ui, Widget, include_image, vec2};
use google_material_symbols::GoogleMaterialSymbols;

use super::{Tab, TabResult, entity_list::EntityListTab, map_list::MapListTab};
use crate::{
    app::SharedState,
    ui::{
        tabs::{activity_list::ActivityListTab, static_list::StaticListTab},
        util::UiExt,
    },
};

pub struct HomeTab;

impl HomeTab {
    pub fn ui(&self, ui: &mut Ui, shared_state: &Arc<SharedState>) -> TabResult {
        static START_TIME: LazyLock<Instant> = LazyLock::new(Instant::now);

        let mut result = TabResult::Continue;

        egui::Image::new(include_image!("../../../assets/ui/bg.png")).paint_at(ui, ui.clip_rect());
        egui::Image::new(include_image!("../../../assets/ui/bg_vignette.png"))
            .tint(Color32::from_white_alpha(
                (64.0 + 64.0 * (START_TIME.elapsed().as_secs_f32()).sin()) as u8,
            ))
            .paint_at(ui, ui.clip_rect());

        #[cfg(debug_assertions)]
        if ui
            .d_button(format!("{} TAG LOOKUP", GoogleMaterialSymbols::Search))
            .clicked()
        {
            use crate::ui::tabs::tag_lookup::TagLookupTab;

            result = TabResult::Open(Tab::TagLookup(TagLookupTab::default()));
        }

        ui.add_space(32.0);
        ui.columns(1, |uis| {
            uis[0].style_mut().text_styles.insert(
                TextStyle::Button,
                FontId::new(32.0, egui::FontFamily::Name("Medium".into())),
            );
            // uis[0].heading("3D");
            // uis[0].add_space(4.0);
            if uis[0]
                .d_button(format!(
                    "{} ACTIVITIES",
                    GoogleMaterialSymbols::StadiaController
                ))
                .clicked()
            {
                result = TabResult::Open(Tab::ActivityList(ActivityListTab::new(shared_state)));
            }
            if uis[0]
                .d_button(format!("{} MAPS", GoogleMaterialSymbols::Map))
                .clicked()
            {
                result = TabResult::Open(Tab::MapList(MapListTab::new(shared_state)));
            }
            if uis[0]
                .d_button(format!("{} ENTITIES", GoogleMaterialSymbols::ChessPawn))
                .clicked()
            {
                // self.added_nodes.push(Tab::DynamicList);
                result =
                    TabResult::Open(Tab::EntityList(Box::new(EntityListTab::new(shared_state))));
            }
            if uis[0]
                .d_button(format!("{} STATICS", GoogleMaterialSymbols::Landscape))
                .clicked()
            {
                result =
                    TabResult::Open(Tab::StaticList(Box::new(StaticListTab::new(shared_state))));
            }

            // uis[1].heading("2D");
            // uis[1].add_space(4.0);
            // uis[1].disable();
            // let _ = uis[1].d_button(format!("{} TEXTURES", GoogleMaterialSymbols::Image));
            // let _ = uis[1].d_button(format!("{} UI", GoogleMaterialSymbols::DesktopWindows));
        });

        egui::TopBottomPanel::bottom("home_links")
            .frame(egui::Frame::NONE)
            .show_separator_line(false)
            .resizable(false)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    egui::Image::new(egui::include_image!("../../../assets/alkahest_256.png"))
                        .fit_to_exact_size(vec2(96.0, 96.0))
                        .ui(ui);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.add_space(12.0);
                        ui.heading("Alkahest");
                        ui.label(
                            RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                                .color(Color32::GRAY),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.image_link(
                            egui::include_image!("../../../assets/ui/github.svg"),
                            vec2(64.0, 64.0),
                            "https://github.com/cohaereo/alkahest",
                        );

                        ui.image_link(
                            egui::include_image!("../../../assets/ui/discord.svg"),
                            vec2(64.0, 48.0),
                            "https://discord.gg/ssGYcJrBUM",
                        )
                        .on_hover_text("Join the Discord server");
                    });
                });
            });

        result
    }
}
