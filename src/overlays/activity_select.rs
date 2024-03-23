use alkahest_data::{
    activity::{SActivity, SDestination},
    map::SBubbleParent,
};
use alkahest_pm::package_manager;
use destiny_pkg::TagHash;
use egui::{ahash::HashMapExt, TextBuffer};
use rustc_hash::FxHashMap;
use tiger_parse::{PackageManagerExt, TigerReadable};

use crate::{
    map::MapList,
    mapload_temporary::{get_map_name, query_activity_maps},
    resources::Resources,
    text::{GlobalStringmap, StringContainer, StringMapShared},
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
    Maps,
}

pub struct ActivityBrowser {
    // pub destinations: Vec<ActivitiesForDestination>,
    pub activity_buckets: Vec<(String, Vec<ActivitiesForDestination>)>,
    pub maps: Vec<(String, Vec<(String, TagHash)>)>,
    show_ambient: bool,
    panel: ActivitySelectPanel,
}

impl ActivityBrowser {
    pub fn new(stringmap_global: &GlobalStringmap) -> Self {
        let destination_hashes = package_manager().get_all_by_reference(SDestination::ID.unwrap());
        // let mut destinations = vec![];
        let mut activity_buckets: FxHashMap<String, Vec<ActivitiesForDestination>> =
            FxHashMap::new();
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

                        activities.push((activity_name, activity_hash));
                    }

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

                    activity_buckets.entry(bucket_name).or_default().push(
                        ActivitiesForDestination {
                            destination_name,
                            destination_code: destination.destination_name.to_string(),
                            activities,
                        },
                    );

                    // destinations.push(ActivitiesForDestination {
                    //     destination_name,
                    //     destination_code: destination.destination_name.to_string(),
                    //     activities,
                    // });
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
            maps,
            show_ambient: false,
            panel: ActivitySelectPanel::Activities,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, resources: &Resources) {
        egui::Window::new("Activities").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.panel,
                    ActivitySelectPanel::Activities,
                    "Activities",
                );
                ui.selectable_value(&mut self.panel, ActivitySelectPanel::Maps, "Maps");
            });
            ui.separator();

            match self.panel {
                ActivitySelectPanel::Activities => self.activities_panel(ctx, ui, resources),
                ActivitySelectPanel::Maps => self.maps_panel(ctx, ui, resources),
            }
        });
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
                            ui.collapsing(
                                destination.destination_code.clone(),
                                // if let Some(destination_name) = &destination.destination_name {
                                //     format!(
                                //         "{} ({destination_name})",
                                //         destination.destination_code
                                //     )
                                // } else {
                                //     destination.destination_code.clone()
                                // },
                                |ui| {
                                    for (activity_name, activity_hash) in &destination.activities {
                                        if !self.show_ambient && activity_name.ends_with("_ambient")
                                        {
                                            continue;
                                        }
                                        let mut activity_name = activity_name.clone();
                                        if activity_name.contains("_ls_") {
                                            activity_name.insert_text(" ", 0);
                                        }
                                        if ui.selectable_label(false, &activity_name).clicked() {
                                            if !set_activity(resources, *activity_hash) {
                                                error!(
                                                    "Failed to query activity maps for \
                                                     {activity_name}"
                                                );
                                                continue;
                                            }
                                        }
                                    }
                                },
                            );
                        }
                    });
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
                                let mut maplist = resources.get_mut::<MapList>().unwrap();

                                maplist.add_map(map_name.clone(), *map_hash);

                                maplist.current_map = maplist.maps.len() - 1;
                            }
                        }
                    });
                }
            });
    }
}
#[derive(Default)]
pub struct CurrentActivity(pub Option<TagHash>);

pub fn set_activity(resources: &Resources, activity_hash: TagHash) -> bool {
    let mut maplist = resources.get_mut::<MapList>().unwrap();
    let stringmap = resources.get::<StringMapShared>().unwrap();
    println!("Attempting to set Activity to {}", activity_hash.0);
    let Ok(maps) = query_activity_maps(activity_hash, &stringmap) else {
        println!("FAILED!");
        return false;
    };
    println!("Success!");
    resources.get_mut::<CurrentActivity>().unwrap().0 = Some(activity_hash);

    maplist.set_maps(&maps);
    true
}

pub fn get_activity_hash(resources: &Resources) -> Option<TagHash> {
    resources.get_mut::<CurrentActivity>().unwrap().0
}
