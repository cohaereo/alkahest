use alkahest_renderer::ecs::common::{Global, Mutable};
use alkahest_renderer::ecs::resources::SelectedEntity;
use alkahest_renderer::ecs::tags::{EntityTag, Tags};
use alkahest_renderer::ecs::utility::Route;
use destiny_pkg::TagHash;
use glam::{Vec2, Vec3};

use crate::gui::activity_select::{get_activity_hash, set_activity};
use crate::maplist::{MapList, MapLoadState};
use alkahest_renderer::camera::tween::Tween;
use alkahest_renderer::camera::Camera;
use alkahest_renderer::resources::Resources;

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
        if let Some(action) = self.current_action.take() {
            if action.is_aborted(resources) {
                self.current_action = None;
                self.action_queue.clear();
            } else if action.is_done(resources) {
                self.current_action = None;
            } else {
                self.current_action = Some(action);
            }
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
        let mut camera = resources.get_mut::<Camera>();

        if let Some(t) = self.t.as_mut() {
            t.reset();
        }
        camera.tween = self.t.take();
    }

    fn is_done(&self, resources: &Resources) -> bool {
        let camera = resources.get::<Camera>();

        camera.tween.as_ref().map_or(true, |t| t.is_finished())
    }

    fn is_aborted(&self, resources: &Resources) -> bool {
        resources
            .get::<Camera>()
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
        let mut maps = resources.get_mut::<MapList>();

        if let Some(new_map) = maps.maps.iter().position(|f| f.hash == self.hash) {
            if maps.current_map_index() != new_map {
                maps.set_current_map(resources, new_map);
            }
        }
    }

    fn is_done(&self, resources: &Resources) -> bool {
        let maps = resources.get::<MapList>();

        maps.current_map()
            .map_or(false, |f| f.load_state == MapLoadState::Loaded)
    }

    fn is_aborted(&self, resources: &Resources) -> bool {
        let maps = resources.get::<MapList>();

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
        if get_activity_hash(resources).is_some_and(|f| f.0 == self.hash.0) {
            return;
        }
        self.aborted = set_activity(resources, self.hash).is_err();
    }

    fn is_done(&self, resources: &Resources) -> bool {
        let map_list = resources.get::<MapList>();

        map_list.count_loaded() > 0 || map_list.count_loading() == 0
    }

    fn is_aborted(&self, _: &Resources) -> bool {
        self.aborted
    }
}

pub struct SpawnRouteAction {
    route: Option<Route>,
}

impl SpawnRouteAction {
    pub fn new(route: Route) -> Self {
        Self { route: Some(route) }
    }
}
impl Action for SpawnRouteAction {
    fn start(&mut self, resources: &Resources) {
        let mut maps = resources.get_mut::<MapList>();

        if let Some(map) = maps.current_map_mut() {
            if let Some(route) = self.route.take() {
                let e = map.scene.spawn((
                    route,
                    Tags::from_iter([EntityTag::Utility, EntityTag::Global]),
                    Mutable,
                    Global,
                ));
                resources.get_mut::<SelectedEntity>().select(e);
            }
        }
    }
    fn is_done(&self, _: &Resources) -> bool {
        true
    }

    fn is_aborted(&self, _: &Resources) -> bool {
        false
    }
}
