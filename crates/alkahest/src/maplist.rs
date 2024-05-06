use std::sync::Arc;

use alkahest_renderer::{
    ecs::{
        common::Global, dynamic_geometry::update_dynamic_model_system,
        static_geometry::update_static_instances_system, Scene,
    },
    loaders::map::load_map,
    renderer::RendererShared,
};
use anyhow::Context;
use destiny_pkg::TagHash;
use hecs::With;
use poll_promise::Promise;

use crate::{
    data::text::StringMapShared, gui::activity_select::CurrentActivity, resources::Resources,
    ApplicationArgs,
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
    pub promise: Option<Box<Promise<anyhow::Result<Scene>>>>,
    pub load_state: MapLoadState,

    pub command_buffer: hecs::CommandBuffer,
    pub scene: Scene,
}

impl Map {
    pub fn update(&mut self) {
        if let Some(promise) = self.promise.take() {
            if promise.ready().is_some() {
                match promise.block_and_take() {
                    Ok(scene) => {
                        self.scene = scene;

                        // TODO(cohae): We can't merge the scenes like this without screwing up the entity IDs
                        // let mut ent_list = vec![];
                        //
                        // // Get all non-global entities
                        // for (entity, _global) in scene
                        //     .query::<Option<&Global>>()
                        //     .iter()
                        //     .filter(|(_, g)| g.is_none())
                        // {
                        //     ent_list.push(entity);
                        // }
                        //
                        // // Insert all entities from the loaded map into the current scene
                        // for entity in ent_list {
                        //     self.scene.spawn(scene.take(entity).ok().unwrap());
                        // }

                        // TODO(cohae): Use scheduler for this
                        update_static_instances_system(&self.scene);
                        update_dynamic_model_system(&self.scene);

                        info!(
                            "Loaded map {} with {} entities",
                            self.name,
                            self.scene.iter().count()
                        );

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

    fn start_load(&mut self, resources: &Resources) {
        if self.load_state != MapLoadState::Unloaded {
            warn!(
                "Attempted to load map {}, but it is already loading or loaded",
                self.hash
            );
            return;
        }

        let renderer = resources.get::<RendererShared>().clone();
        let cli_args = resources.get::<ApplicationArgs>();
        let activity_hash = resources.get_mut::<CurrentActivity>().0;

        info!("Loading map {} '{}'", self.hash, self.name);
        self.promise = Some(Box::new(Promise::spawn_async(load_map(
            renderer,
            self.hash,
            activity_hash,
            !cli_args.no_ambient,
        ))));

        self.load_state = MapLoadState::Loading;
    }
}

#[derive(Default)]
pub struct MapList {
    pub current_map: usize,
    pub previous_map: usize,

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

    pub fn count_loading(&self) -> usize {
        self.maps
            .iter()
            .filter(|m| m.load_state == MapLoadState::Loading)
            .count()
    }

    pub fn count_loaded(&self) -> usize {
        self.maps
            .iter()
            .filter(|m| m.load_state == MapLoadState::Loaded)
            .count()
    }
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
            const LOAD_MAX_PARALLEL: usize = 4;
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

        #[cfg(not(feature = "keep_map_order"))]
        self.maps.sort_by_key(|m| m.name.clone());

        self.current_map = 0;
        self.previous_map = 0;

        // TODO(cohae): Reimplement Discord RPC
        // #[cfg(feature = "discord_rpc")]
        // if let Some(map) = self.current_map_mut() {
        //     discord::set_status_from_mapdata(map);
        // }
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

    pub fn load_all(&mut self, resources: &Resources) {
        for map in self.maps.iter_mut() {
            map.start_load(resources);
        }
    }
}
