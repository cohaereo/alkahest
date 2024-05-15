use alkahest_data::{
    activity::{SActivity, SDestination},
    map::{SBubbleParent, SBubbleParentShallow},
};
use alkahest_pm::package_manager;
use destiny_pkg::TagHash;
use egui::{ahash::HashMapExt, Context, TextBuffer};
use rustc_hash::FxHashMap;
use tiger_parse::{PackageManagerExt, TigerReadable};
use winit::window::Window;

use crate::{
    data::text::{GlobalStringmap, StringContainer, StringMapShared},
    gui::context::{GuiCtx, GuiView, ViewResult},
    maplist::MapList,
    resources::Resources,
};

#[derive(Debug)]
pub struct ActivitiesForDestination {
    pub destination_name: Option<String>,
    pub destination_code: String,
    pub activities: Vec<(String, TagHash)>,
}

#[derive(PartialEq)]
pub enum ActivitySelectPanel {
    Activities,
    Patrols,
    Maps,
}

pub struct ActivityBrowser {
    pub activity_buckets: Vec<(String, Vec<ActivitiesForDestination>)>,
    pub activity_patrols: Vec<(String, TagHash)>,
    pub maps: Vec<(String, Vec<(String, TagHash)>)>,
    show_ambient: bool,
    panel: ActivitySelectPanel,
}

impl ActivityBrowser {
    pub fn new(stringmap_global: &GlobalStringmap) -> Self {
        let destination_hashes = package_manager().get_all_by_reference(SDestination::ID.unwrap());
        let mut activity_buckets: FxHashMap<String, Vec<ActivitiesForDestination>> =
            FxHashMap::new();
        let mut activity_patrols = vec![];

        for (hash, _) in destination_hashes {
            match package_manager().read_tag_struct::<SDestination>(hash) {
                Ok(destination) => {
                    let destination_strings: FxHashMap<u32, String> = {
                        match StringContainer::load(destination.string_container.hash32()) {
                            Ok(sc) => sc.0,
                            Err(e) => {
                                error!("Failed to load string container: {e}");
                                FxHashMap::default()
                            }
                        }
                    };

                    let stringmap = if destination_strings.is_empty() {
                        &stringmap_global.0
                    } else {
                        &destination_strings
                    };

                    let mut activities = vec![];

                    let destination_code = destination.destination_name.to_string();
                    let bucket_name =
                        if let Some(name) = stringmap.get(&destination.location_name.0) {
                            name.clone()
                        } else {
                            destination_code.clone()
                        };

                    let bucket_name = if destination_code.starts_with("gambit_") {
                        "Gambit".to_string()
                    } else if destination_code.starts_with("crucible_") {
                        "Crucible".to_string()
                    } else {
                        bucket_name
                    };

                    let destination_name = stringmap.get(&destination.location_name.0).cloned();

                    for activity in &destination.activities {
                        let activity_code = activity.activity_code.to_string();
                        let activity_code = if activity_code.contains('.') {
                            activity_code.split('.').skip(1).to_owned().collect()
                        } else {
                            activity_code
                        };

                        let Some(activity_hash) = package_manager().get_named_tag(
                            &activity.activity_code.to_string(),
                            SActivity::ID.unwrap(),
                        ) else {
                            error!("Failed to find activity {activity_code}");
                            continue;
                        };

                        let activity_name =
                            if let Some(name) = stringmap.get(&activity.activity_name.0) {
                                format!("{name} ({activity_code})")
                            } else {
                                activity_code
                            };

                        if let Some(base_name) = activity_name.strip_suffix("_freeroam") {
                            activity_patrols
                                .push((format!("{bucket_name} ({base_name})"), activity_hash));
                        }

                        activities.push((activity_name, activity_hash));
                    }

                    activity_buckets.entry(bucket_name).or_default().push(
                        ActivitiesForDestination {
                            destination_name,
                            destination_code: destination.destination_name.to_string(),
                            activities,
                        },
                    );
                }
                Err(e) => {
                    error!("Failed to read SDestination: {}", e);
                    continue;
                }
            }
        }

        let mut maps: FxHashMap<String, Vec<(String, TagHash)>> = FxHashMap::default();

        for (m, _) in package_manager().get_all_by_reference(SBubbleParent::ID.unwrap()) {
            let package_name = package_manager().package_paths[&m.pkg_id()].name.clone();
            let Ok(map_name) = get_map_name(m, stringmap_global) else {
                error!("Failed to get map name for {m}");
                continue;
            };

            maps.entry(package_name).or_default().push((map_name, m));
        }

        let mut maps: Vec<_> = maps.into_iter().collect();
        maps.sort_by(|a, b| a.0.cmp(&b.0));

        let mut activity_buckets: Vec<_> = activity_buckets.into_iter().collect();
        activity_buckets.sort_by(|a, b| a.0.cmp(&b.0));

        Self {
            // destinations,
            activity_buckets,
            activity_patrols,
            maps,
            show_ambient: false,
            panel: ActivitySelectPanel::Activities,
        }
    }

    fn activities_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, resources: &Resources) {
        ui.checkbox(&mut self.show_ambient, "Show ambient activities")
            .on_hover_text(
                "Show ambient activities in the list\nAmbient activities are not accessible \
                 in-game directly, but are used as the base for other activities.",
            );

        egui::ScrollArea::vertical()
            .max_height(ctx.available_rect().height() * 0.9)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for (bucket_name, destinations) in &self.activity_buckets {
                    ui.collapsing(bucket_name, |ui| {
                        for destination in destinations {
                            ui.collapsing(&destination.destination_code, |ui| {
                                for (activity_name, activity_hash) in &destination.activities {
                                    if !self.show_ambient && activity_name.ends_with("_ambient") {
                                        continue;
                                    }
                                    let mut activity_name = activity_name.clone();
                                    if activity_name.contains("_ls_") {
                                        activity_name.insert_text(" ", 0);
                                    }
                                    if ui.selectable_label(false, &activity_name).clicked() {
                                        if let Err(e) = set_activity(resources, *activity_hash) {
                                            error!(
                                                "Failed to set activity \
                                                 {activity_name}/{activity_hash}: {e:?}"
                                            );
                                        }
                                    }
                                }
                            });
                        }
                    });
                }
            });
    }

    fn patrols_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, resources: &Resources) {
        egui::ScrollArea::vertical()
            .max_height(ctx.available_rect().height() * 0.9)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for (patrol_name, activity_hash) in &self.activity_patrols {
                    if ui.selectable_label(false, patrol_name).clicked() {
                        if let Err(e) = set_activity(resources, *activity_hash) {
                            error!(
                                "Failed to set patrol activity {patrol_name}/{activity_hash}: \
                                 {e:?}"
                            );
                        }
                    }
                }
            });
    }

    fn maps_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, resources: &Resources) {
        egui::ScrollArea::vertical()
            .max_height(ctx.available_rect().height() * 0.9)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for (package_name, maps) in &self.maps {
                    ui.collapsing(package_name, |ui| {
                        for (map_name, map_hash) in maps {
                            if ui
                                .selectable_label(false, format!("{map_name} ({map_hash})"))
                                .clicked()
                            {
                                let mut maplist = resources.get_mut::<MapList>();

                                maplist.add_map(map_name.clone(), *map_hash);

                                let new_map = maplist.maps.len() - 1;
                                maplist.set_current_map(new_map);
                            }
                        }
                    });
                }
            });
    }
}

impl GuiView for ActivityBrowser {
    fn draw(
        &mut self,
        ctx: &Context,
        _window: &Window,
        resources: &Resources,
        _gui: &GuiCtx<'_>,
    ) -> Option<ViewResult> {
        egui::Window::new("Activities").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.panel,
                    ActivitySelectPanel::Activities,
                    "Activities",
                );
                ui.selectable_value(&mut self.panel, ActivitySelectPanel::Patrols, "Free Roam");
                ui.selectable_value(&mut self.panel, ActivitySelectPanel::Maps, "Maps");
            });
            ui.separator();

            match self.panel {
                ActivitySelectPanel::Activities => self.activities_panel(ctx, ui, resources),
                ActivitySelectPanel::Patrols => self.patrols_panel(ctx, ui, resources),
                ActivitySelectPanel::Maps => self.maps_panel(ctx, ui, resources),
            }
        });

        None
    }
}

pub fn set_activity(resources: &Resources, activity_hash: TagHash) -> anyhow::Result<()> {
    let mut maplist = resources.get_mut::<MapList>();
    let stringmap = resources.get::<StringMapShared>();
    let maps = query_activity_maps(activity_hash, &stringmap)?;
    resources.get_mut::<CurrentActivity>().0 = Some(activity_hash);
    maplist.set_maps(&maps);
    Ok(())
}

pub fn get_activity_hash(resources: &Resources) -> Option<TagHash> {
    resources.get_mut::<CurrentActivity>().0
}

pub fn get_map_name(map_hash: TagHash, stringmap: &GlobalStringmap) -> anyhow::Result<String> {
    let _span = info_span!("Get map name", %map_hash).entered();
    let map_name = match package_manager().read_tag_struct::<SBubbleParentShallow>(map_hash) {
        Ok(m) => m.map_name,
        Err(e) => {
            anyhow::bail!("Failed to load map {map_hash}: {e}");
        }
    };

    Ok(stringmap.get(map_name))
}

pub fn query_activity_maps(
    activity_hash: TagHash,
    stringmap: &GlobalStringmap,
) -> anyhow::Result<Vec<(TagHash, String)>> {
    let _span = info_span!("Query activity maps").entered();
    let activity: SActivity = package_manager().read_tag_struct(activity_hash)?;
    let mut string_container = StringContainer::default();
    if let Ok(destination) = package_manager().read_tag_struct::<SDestination>(activity.destination)
    {
        if let Ok(sc) = StringContainer::load(destination.string_container) {
            string_container = sc;
        }
    }

    let mut maps = vec![];
    for u1 in &activity.unk50 {
        for map in &u1.map_references {
            let map_name = match package_manager().read_tag_struct::<SBubbleParentShallow>(*map) {
                Ok(m) => m.map_name,
                Err(e) => {
                    error!("Failed to load map {map}: {e:?}");
                    continue;
                }
            };

            let map_name = string_container
                .get(&map_name.0)
                .cloned()
                .unwrap_or_else(|| {
                    // Fall back to global stringmap
                    stringmap.get(map_name)
                });

            maps.push((map.hash32(), map_name));
        }
    }

    Ok(maps)
}

#[derive(Default)]
pub struct CurrentActivity(pub Option<TagHash>);
