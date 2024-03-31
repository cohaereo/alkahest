use destiny_pkg::TagHash;
use glam::{Vec2, Vec3};

use crate::camera::FpsCamera;
use crate::map::{MapList, MapLoadState};
use crate::overlays::activity_select::{get_activity_hash, set_activity};
use crate::render::tween::Tween;
use crate::resources::Resources;
use std::collections::VecDeque;

pub trait Action {
    fn start(&mut self, resources: &Resources);
    fn is_done(&self, resources: &Resources) -> bool;
    fn is_aborted(&self, resources: &Resources) -> bool;
}

#[derive(Default)]
pub struct ActionList {
    action_queue: VecDeque<Box<dyn Action>>,
    current_action: Option<Box<dyn Action>>,
}

impl ActionList {
    pub fn clear_actions(&mut self) {
        self.action_queue.clear();
    }

    pub fn add_action(&mut self, action: impl Action + 'static) {
        self.action_queue.push_back(Box::new(action));
    }

    pub fn process(&mut self, resources: &Resources) {
        let mut clear_current = false;
        let mut clear_all = false;
        if let Some(action) = self.current_action.as_ref() {
            clear_current = action.is_done(resources);
            clear_all = action.is_aborted(resources);
        }
        if clear_current {
            self.current_action = None;
        }
        if clear_all {
            self.current_action = None;
            self.action_queue.clear();
        }

        if self.current_action.as_ref().is_none() {
            self.current_action = self.action_queue.pop_front();
            if let Some(action) = self.current_action.as_mut() {
                action.start(resources);
            }
        }
    }
}

pub struct TweenAction {
    t: Option<Tween>,
}

impl TweenAction {
    pub fn new(
        func: fn(f32) -> f32,
        pos_movement: Option<(Vec3, Vec3)>,
        angle_movement: Option<(Vec2, Vec2)>,
        duration: f32,
    ) -> Self {
        Self {
            t: Some(Tween::new(func, pos_movement, angle_movement, duration)),
        }
    }
}

impl Action for TweenAction {
    fn start(&mut self, resources: &Resources) {
        let mut camera = resources.get_mut::<FpsCamera>().unwrap();

        if let Some(t) = self.t.as_mut() {
            t.reset();
        }
        camera.tween = self.t.take();
    }

    fn is_done(&self, resources: &Resources) -> bool {
        let camera = resources.get::<FpsCamera>().unwrap();

        camera.tween.as_ref().map_or(true, |t| t.is_finished())
    }

    fn is_aborted(&self, resources: &Resources) -> bool {
        resources
            .get::<FpsCamera>()
            .unwrap()
            .tween
            .as_ref()
            .map_or(false, |t| t.is_aborted())
    }
}

pub struct MapSwapAction {
    hash: TagHash,
}

impl MapSwapAction {
    pub fn new(hash: TagHash) -> Self {
        Self { hash }
    }
}

impl Action for MapSwapAction {
    fn start(&mut self, resources: &Resources) {
        let mut maps = resources.get_mut::<MapList>().unwrap();

        if let Some(new_map) = maps.maps.iter().position(|f| f.hash == self.hash) {
            if maps.current_map != new_map {
                (maps.current_map, maps.previous_map) = (new_map, maps.current_map);
                maps.updated = true;
            }
        }
    }

    fn is_done(&self, resources: &Resources) -> bool {
        let maps = resources.get::<MapList>().unwrap();

        maps.current_map()
            .map_or(false, |f| f.load_state == MapLoadState::Loaded)
    }

    fn is_aborted(&self, resources: &Resources) -> bool {
        let maps = resources.get::<MapList>().unwrap();

        maps.current_map()
            .map_or(false, |f| matches!(f.load_state, MapLoadState::Error(_)))
    }
}

pub struct ActivitySwapAction {
    hash: TagHash,
    aborted: bool,
}

impl ActivitySwapAction {
    pub fn new(hash: TagHash) -> Self {
        Self {
            hash,
            aborted: false,
        }
    }
}

impl Action for ActivitySwapAction {
    fn start(&mut self, resources: &Resources) {
        if get_activity_hash(resources).is_some_and(|f|{f.0 == self.hash.0}){
            return
        }
        self.aborted = set_activity(resources, self.hash).is_err();
    }

    fn is_done(&self, _: &Resources) -> bool {
        true
    }

    fn is_aborted(&self, _: &Resources) -> bool {
        self.aborted
    }
}
