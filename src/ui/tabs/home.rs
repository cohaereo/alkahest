use std::sync::Arc;

use egui::{Ui, vec2};
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
        let mut result = TabResult::Continue;

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
            uis[0].heading("3D");
            uis[0].add_space(4.0);
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

                    // let response = ui
                    //     .allocate_response(vec2(64.0, 48.0), Sense::click())
                    //     .on_hover_cursor(egui::CursorIcon::PointingHand);

                    // egui::Image::new(egui::include_image!("../../../assets/ui/discord.svg"))
                    //     .tint(if response.hovered() {
                    //         egui::Color32::LIGHT_GRAY
                    //     } else {
                    //         egui::Color32::DARK_GRAY
                    //     })
                    //     .paint_at(ui, response.rect);

                    // if response.clicked() {
                    //     ui.ctx()
                    //         .open_url(egui::OpenUrl::new_tab("https://discord.gg/ssGYcJrBUM"));
                    // }
                });
            });

        // ui.separator();

        // ui.with_layout(
        //     egui::Layout::top_down_justified(egui::Align::Center),
        //     |ui| {
        //         if ui
        //             .d_button(format!(
        //                 "{} Tag Lookup",
        //                 GoogleMaterialSymbols::Search
        //             ))
        //             .clicked()
        //         {
        //             self.added_nodes
        //                 .push(Tab::TagLookup(TagLookupTab::default()));
        //         }
        //     },
        // );

        result
    }
}
