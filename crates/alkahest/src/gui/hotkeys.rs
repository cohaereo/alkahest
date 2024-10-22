use alkahest_data::occlusion::Aabb;
use alkahest_renderer::{
    camera::{
        tween::{ease_out_exponential, Tween},
        Camera,
    },
    ecs::{
        hierarchy::{Children, Parent},
        resources::SelectedEntity,
        transform::Transform,
        visibility::Visibility,
        Scene,
    },
    renderer::RendererShared,
    util::scene::SceneExt,
};
use bevy_ecs::entity::Entity;
use rustc_hash::FxHashSet;

use crate::{
    maplist::MapList,
    resources::AppResources,
    util::action::{ActionList, TweenAction},
};

pub const SHORTCUT_DELETE: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::SHIFT, egui::Key::Delete);

pub const SHORTCUT_HIDE: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::H);

pub const SHORTCUT_UNHIDE_ALL: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::ALT, egui::Key::H);

pub const SHORTCUT_HIDE_UNSELECTED: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::SHIFT, egui::Key::H);

pub const SHORTCUT_DESELECT: egui::KeyboardShortcut = egui::KeyboardShortcut::new(
    egui::Modifiers::CTRL.plus(egui::Modifiers::SHIFT),
    egui::Key::A,
);

pub const SHORTCUT_FOCUS: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::F);

pub const SHORTCUT_GAZE: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::G);

pub const SHORTCUT_MAP_SWAP: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::I);

pub const SHORTCUT_MAP_PREV: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::PageUp);

pub const SHORTCUT_MAP_NEXT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::PageDown);

pub const SHORTCUT_ADD_ROUTE_NODE_NEXT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::Plus);

pub const SHORTCUT_ADD_ROUTE_NODE_PREV: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::Minus);

pub const SHORTCUT_SELECT_PARENT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::ArrowUp);

pub const SHORTCUT_SELECT_CHILD: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::ArrowDown);

pub const SHORTCUT_SELECT_NEXT_CHILD: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::ArrowRight);

pub const SHORTCUT_SELECT_PREV_CHILD: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::ArrowLeft);

pub fn process_hotkeys(ctx: &egui::Context, resources: &mut AppResources) {
    if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_UNHIDE_ALL)) {
        unhide_all(resources);
    }

    if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_HIDE_UNSELECTED)) {
        hide_unselected(resources);
    }

    if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_DESELECT)) {
        resources.get_mut::<SelectedEntity>().deselect();
    }

    if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_MAP_SWAP)) {
        let mut maplist = resources.get_mut::<MapList>();
        if let Some(prev) = maplist.previous_map {
            maplist.set_current_map(prev);
        }
    }

    if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_MAP_PREV)) {
        resources.get_mut::<MapList>().set_current_map_prev();
    }

    if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_MAP_NEXT)) {
        resources.get_mut::<MapList>().set_current_map_next();
    }

    if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_GAZE)) {
        goto_gaze(resources);
    }

    if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_FOCUS)) {
        focus_selected(resources);
    }

    if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_SELECT_PARENT)) {
        select_parent(resources);
    }

    if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_SELECT_CHILD)) {
        select_child(resources);
    }

    if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_SELECT_NEXT_CHILD)) {
        select_child_offset(resources, true);
    }

    if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_SELECT_PREV_CHILD)) {
        select_child_offset(resources, false);
    }
}

fn focus_selected(resources: &mut AppResources) {
    let mut maps = resources.get_mut::<MapList>();
    let Some(map) = maps.current_map_mut() else {
        return;
    };

    let selected_entity = resources.get::<SelectedEntity>().selected();
    let Some(selected_entity) = selected_entity else {
        return;
    };

    let mut cam = resources.get_mut::<Camera>();
    let bounds = map.scene.get::<Aabb>(selected_entity).cloned();

    let (center, radius) = if let Some(transform) = map.scene.get::<Transform>(selected_entity) {
        if let Some(bounds) = bounds {
            (
                transform.local_to_world().transform_point3(bounds.center()),
                bounds.radius(),
            )
        } else {
            (transform.translation, 1.0)
        }
    } else {
        if let Some(bounds) = bounds {
            (bounds.center(), bounds.radius())
        } else {
            return;
        }
    };

    // Calculate the vertical field of view in radians
    let half_fov_y = (cam.fov() * 0.5).to_radians();

    // Calculate the distance required to fit the sphere in the frustum vertically
    let distance = radius / half_fov_y.tan();

    // Adjust the camera's position to ensure the sphere fits in the frustum
    let target_position = center - cam.forward().normalize() * (distance * 1.75);
    cam.tween = Some(Tween::new(
        ease_out_exponential,
        Some((cam.position(), target_position)),
        None,
        0.5,
    ));
}

fn hide_unselected(resources: &mut AppResources) {
    let mut maps = resources.get_mut::<MapList>();
    let Some(map) = maps.current_map_mut() else {
        return;
    };

    let selected_entity = resources.get::<SelectedEntity>().selected();
    let Some(selected_entity) = selected_entity else {
        return;
    };

    let entity_parents: FxHashSet<Entity> = get_ancestors(&map.scene, selected_entity)
        .into_iter()
        .collect();
    for e in map.scene.iter_entities() {
        if e.id() != selected_entity && !entity_parents.contains(&e.id()) {
            map.commands().entity(e.id()).insert((Visibility::Hidden,));
        }
    }
}

fn get_ancestors(scene: &Scene, entity: Entity) -> Vec<Entity> {
    if let Some(parent) = scene.get::<Parent>(entity) {
        let mut parents = vec![parent.0];
        parents.append(&mut get_ancestors(scene, parent.0));
        parents
    } else {
        vec![]
    }
}

fn unhide_all(resources: &mut AppResources) {
    let mut maps = resources.get_mut::<MapList>();
    if let Some(map) = maps.current_map_mut() {
        map.scene
            .query::<&mut Visibility>()
            .iter_mut(&mut map.scene)
            .for_each(|mut v| *v = Visibility::Visible);
    }
}

fn goto_gaze(resources: &mut AppResources) {
    let camera = resources.get_mut::<Camera>();
    let (d, pos) = resources
        .get::<RendererShared>()
        .data
        .lock()
        .gbuffers
        .depth_buffer_distance_pos_center(&camera);
    if d.is_finite() {
        let mut action_list = resources.get_mut::<ActionList>();
        // Avoid potential weird interactions with routes
        action_list.clear_actions();
        action_list.add_action(TweenAction::new(
            ease_out_exponential,
            Some((camera.position(), pos - camera.forward() * 10.0)),
            None,
            0.7,
        ));
    }
}

fn select_parent(resources: &mut AppResources) {
    let mut selected = resources.get_mut::<SelectedEntity>();
    let mut maps = resources.get_mut::<MapList>();
    if let Some(current) = selected.selected() {
        if let Some(map) = maps.current_map_mut() {
            if let Some(parent) = map.scene.get_parent(current) {
                selected.select(parent);
            }
        }
    }
}

fn select_child(resources: &mut AppResources) {
    let mut selected = resources.get_mut::<SelectedEntity>();
    let mut maps = resources.get_mut::<MapList>();
    if let Some(current) = selected.selected() {
        if let Some(map) = maps.current_map_mut() {
            if let Some(children) = map.scene.get::<Children>(current) {
                if !children.0.is_empty() {
                    selected.select(children.0[0]);
                }
            }
        }
    }
}

fn select_child_offset(resources: &mut AppResources, add: bool) {
    let mut selected = resources.get_mut::<SelectedEntity>();
    let mut maps = resources.get_mut::<MapList>();
    if let Some(current) = selected.selected() {
        if let Some(map) = maps.current_map_mut() {
            if let Some(parent) = map.scene.get_parent(current) {
                if let Some(children) = map.scene.get::<Children>(parent) {
                    let i = children
                        .0
                        .iter()
                        .position(|&ent| ent == current)
                        .unwrap_or(0);
                    let new_i = if add { i + 1 } else { i - 1 };
                    if let Some(next) = children.0.get(new_i) {
                        selected.select(*next);
                    }
                }
            }
        }
    }
}
