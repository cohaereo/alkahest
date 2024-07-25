use alkahest_renderer::{
    camera::{tween::ease_out_exponential, Camera},
    ecs::{common::Hidden, resources::SelectedEntity},
    renderer::RendererShared,
};

use crate::{
    maplist::MapList,
    resources::Resources,
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

pub fn process_hotkeys(ctx: &egui::Context, resources: &mut Resources) {
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
            maplist.set_current_map(resources, prev);
        }
    }

    if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_MAP_PREV)) {
        resources
            .get_mut::<MapList>()
            .set_current_map_prev(resources);
    }

    if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_MAP_NEXT)) {
        resources
            .get_mut::<MapList>()
            .set_current_map_next(resources);
    }

    if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_GAZE)) {
        goto_gaze(resources);
    }
}

fn hide_unselected(resources: &mut Resources) {
    let selected_entity = resources.get::<SelectedEntity>().selected();
    let mut maps = resources.get_mut::<MapList>();
    if let Some(map) = maps.current_map_mut() {
        for e in map.scene.iter() {
            if Some(e.entity()) != selected_entity {
                map.command_buffer.insert_one(e.entity(), Hidden);
            }
        }
    }
}

fn unhide_all(resources: &mut Resources) {
    let mut maps = resources.get_mut::<MapList>();
    if let Some(map) = maps.current_map_mut() {
        for (e, _) in map.scene.query::<&Hidden>().iter() {
            map.command_buffer.remove_one::<Hidden>(e);
        }
    }
}

fn goto_gaze(resources: &mut Resources) {
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
