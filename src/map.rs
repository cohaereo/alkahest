use destiny_pkg::{TagHash, TagHash64};
use glam::Vec4;

use crate::ecs::Scene;

pub struct MapData {
    pub hash: TagHash,
    pub name: String,
    pub scene: Scene,
    pub command_buffer: hecs::CommandBuffer,
}

#[derive(Clone)]
pub struct SimpleLight {
    pub pos: Vec4,
    pub attenuation: Vec4,
}

pub struct MapDataList {
    pub current_map: usize,  // TODO(cohae): Shouldn't be here
    pub previous_map: usize, // TODO(froggy): I guess this too then
    pub updated: bool,
    pub maps: Vec<(TagHash, Option<TagHash64>, MapData)>,
}

impl MapDataList {
    pub fn current_map(&self) -> Option<&(TagHash, Option<TagHash64>, MapData)> {
        if self.maps.is_empty() {
            None
        } else {
            self.maps.get(self.current_map % self.maps.len())
        }
    }

    pub fn current_map_mut(&mut self) -> Option<&mut MapData> {
        if self.maps.is_empty() {
            None
        } else {
            let map_index = self.current_map % self.maps.len();
            self.maps.get_mut(map_index).map(|v| &mut v.2)
        }
    }

    pub fn map_mut(&mut self, i: usize) -> Option<&mut MapData> {
        self.maps.get_mut(i).map(|v| &mut v.2)
    }
}
