pub mod entity_list;
pub mod home;
pub mod map;
pub mod map_list;
pub mod model_list;
pub mod settings;
pub mod static_list;
pub mod tag_lookup;
pub mod test_scene;

use std::{fmt::Display, sync::Arc};

use egui::Margin;
use egui_dock::{DockState, NodeIndex, SurfaceIndex, TabIndex};
use entity_list::EntityListTab;
use google_material_symbols::GoogleMaterialSymbols;
use home::HomeTab;
use map::MapTab;
use map_list::MapListTab;
use tag_lookup::TagLookupTab;

use crate::ui::tabs::{
    settings::SettingsTab, static_list::StaticListTab, test_scene::TestSceneTab,
};

pub enum Tab {
    Home,
    Settings,
    EntityList(Box<EntityListTab>),
    StaticList(Box<StaticListTab>),
    MapList(MapListTab),
    Map(MapTab),
    TestScene(TestSceneTab),
    TagLookup(TagLookupTab),
}

impl Tab {
    pub fn is_fixed(&self) -> bool {
        matches!(self, Tab::Home | Tab::Settings)
    }

    /// Returns an arbitrary key that's unique for the corresponding tab type. Tabs with only 1 instance return 0
    pub fn key(&self) -> u64 {
        match self {
            Tab::Home => 0,
            Tab::Settings => 0,
            Tab::EntityList(_) => 0,
            Tab::StaticList(_) => 0,
            Tab::MapList(_) => 0,
            Tab::Map(tab) => tab.tag.0 as u64,
            Tab::TestScene(_) => 0,
            Tab::TagLookup(_) => 0,
        }
    }
}

impl Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Tab::Settings => GoogleMaterialSymbols::Settings.to_string(),
            Tab::Home => format!("{} Home", GoogleMaterialSymbols::Home),
            Tab::EntityList(_) => format!("{} Entities", GoogleMaterialSymbols::ChessPawn),
            Tab::StaticList(_) => format!("{} Statics", GoogleMaterialSymbols::Landscape),
            Tab::MapList(_) => format!("{} Maps", GoogleMaterialSymbols::Map),
            Tab::Map(tab) => format!("{} ({})", tab.name, tab.tag),
            Tab::TestScene(_) => format!("{} Test Scene", GoogleMaterialSymbols::Experiment),
            Tab::TagLookup(_) => format!("{} Tag Lookup", GoogleMaterialSymbols::Search),
        };

        f.write_str(&s)
    }
}

pub struct TabViewer<'a> {
    pub added_nodes: &'a mut Vec<Tab>,
    pub egui_d3d11: &'a mut egui_d3d11::D3D11Renderer,
    pub shared_state: &'a Arc<crate::app::SharedState>,
}

impl<'a> egui_dock::TabViewer for TabViewer<'a> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.to_string().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        egui::Frame::new()
            .outer_margin(if tab.is_fixed() {
                Margin::symmetric(127, 64)
            } else {
                Margin::ZERO
            })
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| match tab {
                    Tab::Home => {
                        self.process_result(HomeTab.ui(ui, self.shared_state));
                    }
                    Tab::Settings => {
                        SettingsTab::ui(ui, self.shared_state);
                    }
                    Tab::EntityList(tab) => {
                        let res = tab.ui(ui, self.egui_d3d11);
                        self.process_result(res);
                    }
                    Tab::StaticList(tab) => {
                        let res = tab.ui(ui, self.egui_d3d11);
                        self.process_result(res);
                    }
                    Tab::MapList(tab) => {
                        self.process_result(tab.ui(ui));
                    }
                    Tab::Map(tab) => {
                        tab.ui(ui, self.egui_d3d11);
                    }
                    Tab::TestScene(tab) => {
                        tab.ui(ui, self.egui_d3d11);
                    }
                    Tab::TagLookup(data) => {
                        self.process_result(data.ui(ui));
                    }
                });
            });
    }

    fn allowed_in_windows(&self, tab: &mut Self::Tab) -> bool {
        !tab.is_fixed()
    }

    fn is_closeable(&self, tab: &Self::Tab) -> bool {
        !tab.is_fixed()
    }
}

impl<'a> TabViewer<'a> {
    fn process_result(&mut self, result: TabResult) {
        match result {
            TabResult::Continue => {}
            TabResult::Open(tab) => self.added_nodes.push(tab),
        }
    }
}

pub trait DockStateExt<Tab> {
    fn find_tab(
        &self,
        predicate: impl Fn(&Tab) -> bool,
    ) -> Option<(SurfaceIndex, NodeIndex, TabIndex)>;
}

impl<Tab> DockStateExt<Tab> for DockState<Tab> {
    fn find_tab(
        &self,
        predicate: impl Fn(&Tab) -> bool,
    ) -> Option<(SurfaceIndex, NodeIndex, TabIndex)> {
        for (si, surface) in self.iter_surfaces().enumerate() {
            for (ni, node) in surface.iter_nodes().enumerate() {
                for (ti, tab) in node.iter_tabs().enumerate() {
                    if predicate(tab) {
                        return Some((si.into(), ni.into(), ti.into()));
                    }
                }
            }
        }

        None
    }
}

pub enum TabResult {
    Continue,
    Open(Tab),
}
