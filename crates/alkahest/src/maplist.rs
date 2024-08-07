use alkahest_renderer::{
    ecs::{
        common::Global, render::{
            dynamic_geometry::update_dynamic_model_system,
            static_geometry::update_static_instances_system,
        }, resources::SelectedEntity, Scene, SceneInfo
    },
    loaders::map::load_map,
    renderer::RendererShared,
};
use destiny_pkg::TagHash;
use hecs::Entity;
use itertools::Itertools;
use poll_promise::Promise;
use alkahest_data::text::StringContainerShared;
use crate::{
    discord, gui::activity_select::CurrentActivity, resources::Resources, ApplicationArgs,
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
    pub fn update(&mut self, resources: &Resources) {
        if let Some(promise) = self.promise.take() {
            if promise.ready().is_some() {
                match promise.block_and_take() {
                    Ok(mut scene) => {
                        // Move all globals to a temporary scene
                        std::mem::swap(&mut self.scene, &mut scene);
                        self.take_globals(resources, &mut scene);

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
                        error!("Failed to load map {} '{}': {:?}", self.hash, self.name, e);
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

        self.command_buffer.run_on(&mut self.scene);
    }

    /// Remove global entities from the scene and store them in this one
    pub fn take_globals(&mut self, resources: &Resources, source: &mut Scene) {
        let ent_list = source
            .query::<&Global>()
            .iter()
            .map(|(e, _)| e)
            .collect_vec();
        let mut new_selected_entity: Option<Entity> = None;

        {
            let selected_entity = resources.get::<SelectedEntity>().selected();
            for &entity in &ent_list {
                let new_entity = self.scene.spawn(source.take(entity).ok().unwrap());
                if selected_entity.is_some_and(|e|e == entity) {
                    new_selected_entity = Some(new_entity);
                }
            }
        }

        if let Some(new_entity) = new_selected_entity {
            resources.get_mut::<SelectedEntity>().select(new_entity);
        }
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
        let global_strings = resources.get::<StringContainerShared>().clone();

        info!("Loading map {} '{}'", self.hash, self.name);
        self.promise = Some(Box::new(Promise::spawn_async(load_map(
            renderer,
            self.hash,
            activity_hash,
            global_strings,
            !cli_args.no_ambient,
        ))));

        self.load_state = MapLoadState::Loading;
    }
}

#[derive(Default)]
pub struct MapList {
    current_map: usize,
    pub previous_map: Option<usize>,

    pub load_all_maps: bool,

    pub maps: Vec<Map>,
}

impl MapList {
    pub fn current_map_index(&self) -> usize {
        self.current_map
    }

    pub fn current_map(&self) -> Option<&Map> {
        self.maps.get(self.current_map)
    }

    pub fn current_map_mut(&mut self) -> Option<&mut Map> {
        self.maps.get_mut(self.current_map)
    }

    // pub fn get_map_mut(&mut self, index: usize) -> Option<&mut Map> {
    //     self.maps.get_mut(index)
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
            map.update(resources);
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
    pub fn set_maps(&mut self, resources: &Resources, map_hashes: &[(TagHash, String)]) {
        let activity_hash = resources.get_mut::<CurrentActivity>().0;
        self.maps = map_hashes
            .iter()
            .map(|(hash, name)| Map {
                hash: *hash,
                name: name.clone(),
                scene: Scene::new_with_info(activity_hash, *hash),
                ..Default::default()
            })
            .collect();

        #[cfg(not(feature = "keep_map_order"))]
        self.maps.sort_by_key(|m| m.name.clone());

        self.current_map = 0;
        self.previous_map = None;

        #[cfg(feature = "discord_rpc")]
        if let Some(map) = self.current_map() {
            discord::set_activity_from_map(map);
        }
    }

    pub fn add_map(&mut self, resources: &Resources, map_name: String, map_hash: TagHash) {
        if self.maps.is_empty() {
            self.set_maps(resources, &[(map_hash, map_name.clone())])
        } else {
            let activity_hash = resources.get_mut::<CurrentActivity>().0;
            self.maps.push(Map {
                hash: map_hash,
                name: map_name,
                scene: Scene::new_with_info(activity_hash, map_hash),
                ..Default::default()
            });
        }
    }

    pub fn set_current_map(&mut self, resources: &Resources, index: usize) {
        if index >= self.maps.len() {
            warn!(
                "Attempted to set current map to index {}, but there are only {} maps",
                index,
                self.maps.len()
            );
            return;
        }

        self.previous_map = Some(self.current_map);
        self.current_map = index;

        if let Some(previous_map) = self.previous_map {
            if previous_map >= self.maps.len() {
                warn!(
                    "Previous map index {} is out of bounds, not migrating globals",
                    previous_map
                );
                self.previous_map = None;
                return;
            }

            let mut source = std::mem::take(&mut self.maps[previous_map].scene);
            let dest = &mut self.maps[self.current_map];
            dest.take_globals(resources, &mut source);
            self.maps[previous_map].scene = source;
        }

        #[cfg(feature = "discord_rpc")]
        if let Some(map) = self.current_map() {
            discord::set_activity_from_map(map);
        }
    }

    pub fn set_current_map_next(&mut self, resources: &Resources) {
        if self.current_map + 1 < self.maps.len() {
            self.set_current_map(resources, self.current_map + 1)
        }
    }

    pub fn set_current_map_prev(&mut self, resources: &Resources) {
        if self.current_map > 0 && self.maps.len() >= 1 {
            self.set_current_map(resources, self.current_map - 1)
        }
    }
}
