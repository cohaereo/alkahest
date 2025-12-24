use std::sync::Arc;

use egui::Ui;
use google_material_symbols::GoogleMaterialSymbols;

use super::{Tab, TabResult, entity_list::EntityListTab, map_list::MapListTab};
use crate::{app::SharedState, ui::util::UiExt};

pub struct HomeTab;

impl HomeTab {
    pub fn ui(&self, ui: &mut Ui, shared_state: &Arc<SharedState>) -> TabResult {
        let mut result = TabResult::Continue;
        ui.add_space(32.0);
        ui.columns(2, |uis| {
            uis[0].heading("3D");
            uis[0].add_space(4.0);
            if uis[0]
                .d_button(format!("{} ENTITIES", GoogleMaterialSymbols::DeployedCode))
                .clicked()
            {
                // self.added_nodes.push(Tab::DynamicList);
                result = TabResult::Open(Tab::EntityList(Box::new(EntityListTab::new())));
            }
            if uis[0]
                .d_button(format!("{} MAPS", GoogleMaterialSymbols::Map))
                .clicked()
            {
                result = TabResult::Open(Tab::MapList(MapListTab::new(&shared_state.strings)));
            }
            uis[0].disable();
            let _ = uis[0].d_button(format!("{} STATICS", GoogleMaterialSymbols::Landscape));

            uis[1].heading("2D");
            uis[1].add_space(4.0);
            uis[1].disable();
            let _ = uis[1].d_button(format!("{} TEXTURES", GoogleMaterialSymbols::Image));
            let _ = uis[1].d_button(format!("{} UI", GoogleMaterialSymbols::DesktopWindows));
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
