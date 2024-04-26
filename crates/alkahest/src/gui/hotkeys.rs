use crate::resources::Resources;

pub const SHORTCUT_DELETE: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::SHIFT, egui::Key::Delete);

pub const SHORTCUT_HIDE: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::H);

pub const SHORTCUT_UNHIDE_ALL: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::ALT, egui::Key::H);

pub const SHORTCUT_HIDE_UNSELECTED: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::SHIFT, egui::Key::H);

pub const SHORTCUT_FOCUS: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::F);

pub const SHORTCUT_GAZE: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::G);

pub const SHORTCUT_MAP_SWAP: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::I);

pub fn process_hotkeys(ctx: &egui::Context, resources: &mut Resources) {
    if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_UNHIDE_ALL)) {
        unhide_all(resources);
    }

    if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_HIDE_UNSELECTED)) {
        hide_unselected(resources);
    }
}

fn hide_unselected(_resources: &mut Resources) {
    // TODO(cohae): Reimplement once we have maps again
    // let selected_entity = resources.get::<SelectedEntity>().select;
    // let mut maps = resources.get_mut::<MapList>();
    // if let Some(map) = maps.current_map_mut() {
    //     for (entity, vis) in map.scene.query::<Option<&mut Visible>>().iter() {
    //         if Some(entity) != selected_entity {
    //             if let Some(vis) = vis {
    //                 vis.0 = false;
    //             } else {
    //                 map.command_buffer.insert_one(entity, Visible(false));
    //             }
    //         }
    //     }
    // }
}

fn unhide_all(_resources: &mut Resources) {
    // TODO(cohae): Reimplement once we have maps again
    // let maps = resources.get::<MapList>();
    // if let Some(map) = maps.current_map() {
    //     for (_, vis) in map.scene.query::<&mut Visible>().iter() {
    //         vis.0 = true;
    //     }
    // }
}
