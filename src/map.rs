use std::sync::Arc;

use anyhow::Context;
use destiny_pkg::TagHash;
use poll_promise::Promise;
use rustc_hash::FxHashMap;

use crate::{
    discord,
    ecs::{components::Global, Scene},
    mapload_temporary::{self, LoadMapData},
    render::{dcs::DcsShared, renderer::RendererShared, EntityRenderer},
    resources::Resources,
    Args, StringMapShared,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MapLoadState {
    #[default]
    Unloaded,
    Loading,
    Loaded,
    Error(String),
}

#[derive(Default)]
pub struct Map {
    pub hash: TagHash,
    pub name: String,
    pub promise: Option<Box<Promise<anyhow::Result<LoadMapData>>>>,
    pub load_state: MapLoadState,
    pub command_buffer: hecs::CommandBuffer,

    pub scene: Scene,
    // TODO(cohae): Move this to asset management
    pub entity_renderers: FxHashMap<u64, EntityRenderer>,
}

impl Map {
    pub fn update(&mut self) {
        // TODO(cohae): This seems dirty
        if let Some(promise) = self.promise.take() {
            if promise.ready().is_some() {
                match promise.block_and_take() {
                    Ok(mut map) => {
                        let mut ent_list = vec![];

                        // Get all non-global entities
                        for (entity, global) in map.scene.query::<Option<&Global>>().iter() {
                            if !global.map_or(false, |g| g.0) {
                                ent_list.push(entity);
                            }
                        }

                        // Insert all entities from the loaded map into the current scene
                        for entity in ent_list {
                            self.scene.spawn(map.scene.take(entity).ok().unwrap());
                        }

                        self.entity_renderers = map.entity_renderers;
                        self.load_state = MapLoadState::Loaded;
                    }
                    Err(e) => {
                        self.load_state = MapLoadState::Error(format!("{:?}", e));
                    }
                }
            } else {
                self.promise = Some(promise);
                self.load_state = MapLoadState::Loading;
            }
        }
        // else {
        //     self.load_state = MapLoadState::Unloaded;
        // }
    }

    fn start_load(&mut self, resources: &crate::Resources) {
        if self.load_state != MapLoadState::Unloaded {
            warn!(
                "Attempted to load map {}, but it is already loading or loaded",
                self.hash
            );
            return;
        }
        let dcs = Arc::clone(&resources.get::<DcsShared>().unwrap());
        let renderer = Arc::clone(&resources.get::<RendererShared>().unwrap());
        let cli_args = resources.get::<Args>().unwrap();
        let stringmap = Arc::clone(&resources.get::<StringMapShared>().unwrap());

        let activity_hash = cli_args.activity.as_ref().map(|a| {
            TagHash(u32::from_be(
                u32::from_str_radix(a, 16)
                    .context("Invalid activity hash format")
                    .unwrap(),
            ))
        });

        info!("Loading map {} '{}'", self.hash, self.name);
        self.promise = Some(Box::new(Promise::spawn_async(
            mapload_temporary::load_map_scene(
                dcs,
                renderer,
                self.hash,
                stringmap,
                activity_hash,
                !cli_args.no_ambient,
            ),
        )));

        self.load_state = MapLoadState::Loading;
    }
}

#[derive(Default)]
pub struct MapList {
    pub current_map: usize,
    pub previous_map: usize,

    // TODO(cohae): What is this used for?
    pub updated: bool,
    pub load_all_maps: bool,

    pub maps: Vec<Map>,
}

impl MapList {
    pub fn current_map(&self) -> Option<&Map> {
        self.maps.get(self.current_map)
    }

    pub fn current_map_mut(&mut self) -> Option<&mut Map> {
        self.maps.get_mut(self.current_map)
    }

    pub fn get_map_mut(&mut self, index: usize) -> Option<&mut Map> {
        self.maps.get_mut(index)
    }

    // pub fn get_index(&self, tag: TagHash) -> Option<usize> {
    //     self.maps.iter().position(|m| m.hash == tag)
    // }
}

impl MapList {
    pub fn update_maps(&mut self, resources: &Resources) {
        for (i, map) in self.maps.iter_mut().enumerate() {
            map.update();
            if i == self.current_map && map.load_state == MapLoadState::Unloaded {
                map.start_load(resources);
            }
        }

        if self.load_all_maps {
            // TODO(cohae): Loading multiple maps at once doesn't play well with the current asset management system
            const LOAD_MAX_PARALLEL: usize = 1;
            let mut loaded = 0;
            for map in self.maps.iter_mut() {
                if loaded >= LOAD_MAX_PARALLEL {
                    break;
                }

                if map.load_state == MapLoadState::Loading {
                    loaded += 1;
                }

                if map.load_state == MapLoadState::Unloaded {
                    map.start_load(resources);
                    loaded += 1;
                }
            }
        }
    }

    /// Populates the map list and begins loading the first map
    /// Overwrites the current map list
    pub fn set_maps(&mut self, map_hashes: &[(TagHash, String)]) {
        self.maps = map_hashes
            .iter()
            .map(|(hash, name)| Map {
                hash: *hash,
                name: name.clone(),
                ..Default::default()
            })
            .collect();

        self.updated = true;
        self.current_map = 0;
        self.previous_map = 0;

        #[cfg(feature = "discord_rpc")]
        if let Some(map) = self.current_map_mut() {
            discord::set_status_from_mapdata(map);
        }
    }

    pub fn add_map(&mut self, map_name: String, map_hash: TagHash) {
        if self.maps.is_empty() {
            self.set_maps(&[(map_hash, map_name.clone())])
        } else {
            self.maps.push(Map {
                hash: map_hash,
                name: map_name,
                ..Default::default()
            });
        }
    }

    // pub fn load_all(&mut self, resources: &crate::Resources) {
    //     for map in self.maps.iter_mut() {
    //         map.start_load(resources);
    //     }
    // }
}

// impl Default for MapList {
//     fn default() -> Self {
//         Self {
//             current_map: 0,
//             previous_map: 0,
//             updated: false,
//             load_all_maps: false,
//             maps: vec![],
//         }
//     }
// }
