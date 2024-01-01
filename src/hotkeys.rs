use crate::{
    ecs::{components::Visible, resources::SelectedEntity},
    map::MapDataList,
    resources::Resources,
};

fn alt_only(modifiers: egui::Modifiers) -> bool {
    modifiers.alt && !modifiers.ctrl && !modifiers.command && !modifiers.shift
}

pub fn process_hotkeys(ctx: &egui::Context, resources: &mut Resources) {
    if ctx.input(|i| alt_only(i.modifiers) && i.key_pressed(egui::Key::H)) {
        unhide_all(resources);
    }

    if ctx.input(|i| i.modifiers.shift_only() && i.key_pressed(egui::Key::H)) {
        hide_unselected(resources);
    }
}

fn hide_unselected(resources: &mut Resources) {
    let selected_entity = resources.get::<SelectedEntity>().unwrap().0;
    if let Some(mut maps) = resources.get_mut::<MapDataList>() {
        if let Some(map) = maps.current_map_mut() {
            let mut add_vis_to = vec![];
            for (entity, vis) in map.scene.query::<Option<&mut Visible>>().iter() {
                if Some(entity) != selected_entity {
                    if let Some(vis) = vis {
                        vis.0 = false;
                    } else {
                        add_vis_to.push(entity);
                    }
                }
            }

            for entity in add_vis_to {
                map.scene.insert_one(entity, Visible(false)).ok();
            }
        }
    }
}

fn unhide_all(resources: &mut Resources) {
    if let Some(maps) = resources.get::<MapDataList>() {
        if let Some((_, _, map)) = maps.current_map() {
            for (_, vis) in map.scene.query::<&mut Visible>().iter() {
                vis.0 = true;
            }
        }
    }
}
