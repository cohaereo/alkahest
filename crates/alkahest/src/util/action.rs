use std::collections::VecDeque;

use alkahest_renderer::{
    camera::{get_look_angle, tween::Tween, Camera},
    ecs::{
        common::{Global, Mutable, RenderCommonBundle},
        hierarchy::{Children, Parent},
        resources::SelectedEntity,
        tags::{EntityTag, NodeFilter, Tags},
        transform::{Transform, TransformFlags},
        utility::{Route, RouteHolder, RouteNode, Utility},
    },
    resources::AppResources,
};
use bevy_ecs::entity::Entity;
use destiny_pkg::TagHash;
use glam::{Vec2, Vec3};

use crate::{
    gui::activity_select::{get_activity_hash, set_activity},
    maplist::{MapList, MapLoadState},
};

pub trait Action {
    fn start(&mut self, resources: &AppResources);
    fn is_done(&self, resources: &AppResources) -> bool;
    fn is_aborted(&self, resources: &AppResources) -> bool;
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

    pub fn process(&mut self, resources: &AppResources) {
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

        self.add_buffered_actions(resources);

        if self.current_action.as_ref().is_none() {
            self.current_action = self.action_queue.pop_front();
            if let Some(action) = self.current_action.as_mut() {
                action.start(resources);
            }
        }
    }

    fn add_buffered_actions(&mut self, resources: &AppResources) {
        let mut buffer = resources.get_mut::<ActionBuffer>();
        self.action_queue.append(&mut buffer.buffer_queue);
    }
}

#[derive(Default)]
pub struct ActionBuffer {
    buffer_queue: VecDeque<Box<dyn Action>>,
}

impl ActionBuffer {
    pub fn buffer_action(&mut self, action: impl Action + 'static) {
        self.buffer_queue.push_back(Box::new(action));
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
    fn start(&mut self, resources: &AppResources) {
        let mut camera = resources.get_mut::<Camera>();

        if let Some(t) = self.t.as_mut() {
            t.reset();
        }
        camera.tween = self.t.take();
    }

    fn is_done(&self, resources: &AppResources) -> bool {
        let camera = resources.get::<Camera>();

        camera.tween.as_ref().map_or(true, |t| t.is_finished())
    }

    fn is_aborted(&self, resources: &AppResources) -> bool {
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
    fn start(&mut self, resources: &AppResources) {
        let mut maps = resources.get_mut::<MapList>();

        if let Some(new_map) = maps.maps.iter().position(|f| f.hash == self.hash) {
            if maps.current_map_index() != new_map {
                maps.set_current_map(new_map);
            }
        }
    }

    fn is_done(&self, resources: &AppResources) -> bool {
        let maps = resources.get::<MapList>();

        maps.current_map()
            .map_or(false, |f| f.load_state == MapLoadState::Loaded)
    }

    fn is_aborted(&self, resources: &AppResources) -> bool {
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
    fn start(&mut self, resources: &AppResources) {
        if get_activity_hash(resources).is_some_and(|f| f.0 == self.hash.0) {
            return;
        }
        self.aborted = set_activity(resources, self.hash).is_err();
    }

    fn is_done(&self, resources: &AppResources) -> bool {
        let map_list = resources.get::<MapList>();

        map_list.count_loaded() > 0 || map_list.count_loading() == 0
    }

    fn is_aborted(&self, _: &AppResources) -> bool {
        self.aborted
    }
}

pub struct SpawnRouteAction {
    route: Option<RouteHolder>,
}

impl SpawnRouteAction {
    pub fn new(route: RouteHolder) -> Self {
        Self { route: Some(route) }
    }
}
impl Action for SpawnRouteAction {
    fn start(&mut self, resources: &AppResources) {
        let mut maps = resources.get_mut::<MapList>();

        if let Some(map) = maps.current_map_mut() {
            if let Some(route) = self.route.take() {
                let parent = map.scene.spawn_empty().id();
                let mut children = vec![];
                for node in route.path {
                    let e = map
                        .scene
                        .spawn((
                            Parent(parent),
                            Transform {
                                translation: node.pos,
                                flags: TransformFlags::IGNORE_ROTATION
                                    | TransformFlags::IGNORE_SCALE,
                                ..Default::default()
                            },
                            RouteNode {
                                map_hash: node.map_hash,
                                is_teleport: node.is_teleport,
                            },
                            if let Some(label) = node.label {
                                RouteNode::label(&label)
                            } else {
                                RouteNode::default_label()
                            },
                            RouteNode::icon(),
                            NodeFilter::Utility,
                            Tags::from_iter([EntityTag::Utility, EntityTag::Global]),
                            Mutable,
                            Global,
                            RenderCommonBundle::default(),
                        ))
                        .id();
                    children.push(e);
                }
                map.scene.entity_mut(parent).insert((
                    Children::from_slice(&children),
                    Route {
                        activity_hash: route.activity_hash,
                        ..Default::default()
                    },
                    Route::icon(),
                    Route::default_label(),
                    NodeFilter::Utility,
                    Tags::from_iter([EntityTag::Utility, EntityTag::Global]),
                    Mutable,
                    Global,
                    RenderCommonBundle::default(),
                ));
                resources.get_mut::<SelectedEntity>().select(parent);
                if let Some(route) = map.scene.get::<Route>(parent) {
                    route.fixup_visiblity(&map.scene, &mut map.commands(), parent);
                }
            }
        }
    }
    fn is_done(&self, _: &AppResources) -> bool {
        true
    }

    fn is_aborted(&self, _: &AppResources) -> bool {
        false
    }
}

pub struct FollowAction {
    route_ent: Entity,
    traverse_from: Option<Entity>,
}

impl FollowAction {
    pub fn new(route_ent: Entity, traverse_from: Option<Entity>) -> Self {
        Self {
            route_ent,
            traverse_from,
        }
    }
}

impl Action for FollowAction {
    fn start(&mut self, resources: &AppResources) {
        let camera = resources.get::<Camera>();
        let mut maps = resources.get_mut::<MapList>();

        if let Some(map) = maps.current_map_mut() {
            let scene = &mut map.scene;
            let camera_offset = Vec3::Z;
            let mut buffer = resources.get_mut::<ActionBuffer>();
            const DEGREES_PER_SEC: f32 = 360.0;
            const METERS_PER_SEC: f32 = 18.0;

            let route_ref = scene.entity(self.route_ent);

            let Some(route) = route_ref.get::<Route>() else {
                return;
            };

            if let Some(children) = route_ref.get::<Children>() {
                if children.0.is_empty() {
                    return;
                }

                let start_index = match self.traverse_from {
                    Some(e) => children.0.iter().position(|&ent| ent == e).unwrap_or(0),
                    None => 0,
                };

                let start_node_ent = scene.entity(children.0[start_index]);
                let start_pos = match start_node_ent.get::<Transform>() {
                    Some(t) => t.translation,
                    None => return,
                };
                let start_hash = match start_node_ent.get::<RouteNode>() {
                    Some(n) => n.map_hash,
                    None => return,
                };

                let mut old_pos = start_pos + camera_offset;
                let mut old_orient = camera.get_look_angle(old_pos);
                buffer.buffer_action(TweenAction::new(
                    |x| x,
                    Some((camera.position(), old_pos)),
                    Some((camera.view_angle(), old_orient)),
                    1.0,
                ));

                if let Some(hash) = start_hash {
                    buffer.buffer_action(MapSwapAction::new(hash));
                }
                for node_e in children.0.iter().skip(start_index + 1) {
                    let Some(pos) = scene.get::<Transform>(*node_e) else {
                        return;
                    };
                    let Some(node) = scene.get::<RouteNode>(*node_e) else {
                        return;
                    };

                    info!("Attempting to get next for {}", node_e);
                    let new_pos = pos.translation + camera_offset;
                    let new_orient = get_look_angle(old_orient, old_pos, new_pos);
                    //TODO Not sure why this isn't working right
                    // let angle_dif = get_look_angle_difference(old_orient, old_pos, new_pos);
                    // Using a silly approximation to look ok.
                    let angle_delta = (old_orient - new_orient).abs();
                    let angle_dif = (angle_delta.x % 360.0).max(angle_delta.y % 360.0);
                    buffer.buffer_action(TweenAction::new(
                        |x| x,
                        None,
                        Some((old_orient, new_orient)),
                        angle_dif / (DEGREES_PER_SEC * route.speed_multiplier),
                    ));
                    old_orient = new_orient;
                    buffer.buffer_action(TweenAction::new(
                        |x| x,
                        Some((old_pos, new_pos)),
                        None,
                        if node.is_teleport {
                            route.scale * 0.1
                        } else {
                            route.scale * old_pos.distance(new_pos)
                                / (METERS_PER_SEC * route.speed_multiplier)
                        },
                    ));
                    if let Some(hash) = node.map_hash {
                        buffer.buffer_action(MapSwapAction::new(hash));
                    }
                    old_pos = new_pos;
                }
            }
        }
    }

    fn is_done(&self, _: &AppResources) -> bool {
        true
    }

    fn is_aborted(&self, _: &AppResources) -> bool {
        // Clear the action list, then run buffered actions
        true
    }
}
