use destiny_pkg::TagHash;

pub mod audio;
pub mod common;
pub mod hierarchy;
pub mod map;
pub mod render;
pub mod resources;
pub mod scene_ext;
pub mod tags;
pub mod transform;
pub mod utility;

pub type Scene = hecs::World;
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
        let mut scene = Scene::new();

        scene.add_map_info(activity_hash, map_hash);
        scene
    }
    fn add_map_info(&mut self, activity_hash: Option<TagHash>, map_hash: TagHash) {
        if self.query::<&MapInfo>().iter().next().is_none() {
            self.spawn((MapInfo {
                activity_hash,
                map_hash,
            },));
        }
    }
    fn get_map_hash(&self) -> Option<TagHash> {
        self.query::<&MapInfo>()
            .iter()
            .next()
            .map(|(_, i)| i.map_hash)
    }
    fn get_activity_hash(&self) -> Option<TagHash> {
        self.query::<&MapInfo>()
            .iter()
            .next()
            .and_then(|(_, i)| i.activity_hash)
    }
}
