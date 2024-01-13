use std::fmt::Display;

use egui::Color32;
use hecs::Entity;
use nohash_hasher::IntSet;

use crate::{icons::ICON_WEB, overlays::UiExt, util::text::name_to_color};

use super::Scene;

#[derive(strum::EnumIter, Hash, PartialEq, Eq)]
pub enum EntityTag {
    Activity,
    Ambient,
    Global,
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

impl Display for EntityTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityTag::Activity => write!(f, "Activity"),
            EntityTag::Ambient => write!(f, "Ambient"),
            EntityTag::Global => write!(f, "{} Global", ICON_WEB),
            EntityTag::Havok => write!(f, "Havok"),
            EntityTag::Utility => write!(f, "Utility"),
            EntityTag::User => write!(f, "User"),
        }
    }
}

#[derive(Default)]
pub struct Tags(pub IntSet<EntityTag>);

impl Tags {
    pub fn from_iter(arr: impl IntoIterator<Item = EntityTag>) -> Self {
        Self(arr.into_iter().collect())
    }

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

    scene.insert_one(ent, Tags::from_iter([tag])).ok();
}

pub fn remove_tag(scene: &mut Scene, ent: Entity, tag: EntityTag) {
    if let Ok(Some(mut e)) = scene.entity(ent).map(|e| e.get::<&mut Tags>()) {
        e.0.remove(&tag);
    }
}
