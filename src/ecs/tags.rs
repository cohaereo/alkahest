use egui::Color32;
use hecs::Entity;
use nohash_hasher::IntSet;

use crate::overlays::{chip::name_to_color, UiExt};

use super::Scene;

#[derive(strum::Display, strum::EnumIter, Hash, PartialEq, Eq)]
pub enum EntityTag {
    Activity,
    Ambient,
    Havok,
    Utility,
    User,
}

impl nohash_hasher::IsEnabled for EntityTag {}

impl EntityTag {
    pub fn color(&self) -> Color32 {
        match self {
            EntityTag::Havok => Color32::from_rgb(253, 185, 10),
            u => name_to_color(&u.to_string()),
        }
    }
}

#[derive(Default)]
pub struct Tags(pub IntSet<EntityTag>);

impl Tags {
    pub fn insert(&mut self, tag: EntityTag) {
        self.0.insert(tag);
    }

    // pub fn remove(&mut self, tag: EntityTag) {
    //     self.0.remove(&tag);
    // }

    pub fn ui_chips(&self, ui: &mut egui::Ui) {
        for tag in self.0.iter() {
            ui.chip_with_color(tag.to_string(), tag.color());
        }
    }
}

pub fn insert_tag(scene: &mut Scene, ent: Entity, tag: EntityTag) {
    if let Ok(Some(mut e)) = scene.entity(ent).map(|e| e.get::<&mut Tags>()) {
        e.insert(tag);
        return;
    }

    scene
        .insert_one(ent, Tags([tag].into_iter().collect()))
        .ok();
}
