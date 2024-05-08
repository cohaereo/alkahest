use std::fmt::Display;

use hecs::{Entity, EntityRef};
use rustc_hash::FxHashSet;
use tiger_parse::FnvHash;

use super::Scene;
use crate::util::color::Color;

#[derive(strum::EnumIter, Hash, PartialEq, Eq)]
pub enum EntityTag {
    Activity,
    Ambient,
    Global,
    Havok,
    Utility,
    User,
}

pub const FNV1_BASE: u32 = 0x811c9dc5;
pub const FNV1_PRIME: u32 = 0x01000193;
fn fnv1(data: &[u8]) -> FnvHash {
    data.iter().fold(FNV1_BASE, |acc, b| {
        acc.wrapping_mul(FNV1_PRIME) ^ (*b as u32)
    })
}

fn name_to_color(name: &str) -> Color {
    let hash = fnv1(name.as_bytes());
    let r = (hash & 0xFF) as u8;
    let g = ((hash >> 8) & 0xFF) as u8;
    let b = ((hash >> 16) & 0xFF) as u8;
    [r, g, b, 255].into()
}

impl EntityTag {
    pub fn color(&self) -> Color {
        match self {
            EntityTag::Havok => [253, 185, 10, 255].into(),
            u => name_to_color(&u.to_string()),
        }
    }
}

impl Display for EntityTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityTag::Activity => write!(f, "Activity"),
            EntityTag::Ambient => write!(f, "Ambient"),
            EntityTag::Global => write!(f, "Global"),
            EntityTag::Havok => write!(f, "Havok"),
            EntityTag::Utility => write!(f, "Utility"),
            EntityTag::User => write!(f, "User"),
        }
    }
}

#[derive(Default)]
pub struct Tags(pub FxHashSet<EntityTag>);

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
