use bevy_ecs::system::Resource;
use destiny_pkg::TagHash;
use resources::SelectedEntity;

pub mod audio;
pub mod common;
pub mod hierarchy;
pub mod map;
pub mod render;
pub mod resources;
pub mod tags;
pub mod transform;
pub mod utility;

pub type Scene = bevy_ecs::world::World;

/// Creates a new scene with some default resources (Camera, SelectedEntity, etc)
pub fn new_scene() -> Scene {
    let mut scene = Scene::new();
    scene.insert_resource(SelectedEntity::default());
    scene
}

#[derive(Resource)]
pub struct MapInfo {
    pub activity_hash: Option<TagHash>,
    pub map_hash: TagHash,
}

pub trait SceneInfo {
    fn new_with_info(activity_hash: Option<TagHash>, map_hash: TagHash) -> Self;
    fn add_map_info(&mut self, activity_hash: Option<TagHash>, map_hash: TagHash);
    fn get_map_hash(&self) -> Option<TagHash>;
    fn get_activity_hash(&self) -> Option<TagHash>;
}
impl SceneInfo for Scene {
    fn new_with_info(activity_hash: Option<TagHash>, map_hash: TagHash) -> Self {
        let mut scene = new_scene();

        scene.add_map_info(activity_hash, map_hash);
        scene
    }
    fn add_map_info(&mut self, activity_hash: Option<TagHash>, map_hash: TagHash) {
        if self.get_resource::<MapInfo>().is_none() {
            self.insert_resource(MapInfo {
                activity_hash,
                map_hash,
            });
        }
    }
    fn get_map_hash(&self) -> Option<TagHash> {
        self.get_resource::<MapInfo>().map(|i| i.map_hash)
    }
    fn get_activity_hash(&self) -> Option<TagHash> {
        self.get_resource::<MapInfo>().and_then(|i| i.activity_hash)
    }
}
