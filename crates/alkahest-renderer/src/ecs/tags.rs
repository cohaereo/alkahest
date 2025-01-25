use std::fmt::Display;

use bevy_ecs::{component::Component, entity::Entity};
use rustc_hash::FxHashSet;
use tiger_parse::FnvHash;

use super::Scene;
use crate::{
    icons::{
        ICON_ACCOUNT_CONVERT, ICON_CHESS_PAWN, ICON_CUBE, ICON_DROPBOX, ICON_HELP,
        ICON_LIGHTBULB_ON, ICON_PINE_TREE, ICON_REPLY, ICON_SKULL, ICON_SPHERE, ICON_STICKER,
        ICON_TAG, ICON_TOOLBOX, ICON_VOLUME_HIGH, ICON_WEATHER_PARTLY_CLOUDY,
    },
    util::{color::Color, scene::EntityWorldMutExt},
};

pub type NodeFilterSet = FxHashSet<NodeFilter>;

#[derive(Component, strum::EnumIter, strum::Display, Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum NodeFilter {
    Entity,
    RespawnPoint,
    Light,
    Sound,
    Decorator,
    SkyObject,
    Cubemap,
    Static,
    Decal,

    InstakillBarrier,
    TurnbackBarrier,
    PlayerContainmentVolume,
    NamedArea,
    Unknown80809121,

    Utility,

    Unknown,
}

impl NodeFilter {
    pub fn icon(&self) -> char {
        match self {
            NodeFilter::Entity => ICON_CHESS_PAWN,
            NodeFilter::RespawnPoint => ICON_ACCOUNT_CONVERT,
            NodeFilter::Light => ICON_LIGHTBULB_ON,
            NodeFilter::Sound => ICON_VOLUME_HIGH,
            NodeFilter::Decorator => ICON_PINE_TREE,
            NodeFilter::SkyObject => ICON_WEATHER_PARTLY_CLOUDY,
            NodeFilter::Cubemap => ICON_SPHERE,
            NodeFilter::Static => ICON_CUBE,
            NodeFilter::Decal => ICON_STICKER,
            NodeFilter::InstakillBarrier => ICON_SKULL,
            NodeFilter::TurnbackBarrier => ICON_REPLY,
            NodeFilter::PlayerContainmentVolume => ICON_DROPBOX,
            NodeFilter::NamedArea => ICON_TAG,
            NodeFilter::Unknown80809121 => ICON_HELP,
            NodeFilter::Utility => ICON_TOOLBOX,
            NodeFilter::Unknown => ICON_HELP,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            NodeFilter::Entity => Color::WHITE,
            NodeFilter::RespawnPoint => Color::from_srgba_unmultiplied(220, 20, 20, 255),
            NodeFilter::Light => Color::from_srgba_unmultiplied(255, 255, 0, 255),
            NodeFilter::Sound => Color::from_srgba_unmultiplied(0, 192, 0, 255),
            NodeFilter::Decorator => Color::from_srgba_unmultiplied(80, 210, 80, 255),
            NodeFilter::SkyObject => Color::from_srgba_unmultiplied(0xAD, 0xD8, 0xE6, 255),
            NodeFilter::Cubemap => Color::from_srgba_unmultiplied(50, 255, 50, 255),
            NodeFilter::Static => Color::WHITE,
            NodeFilter::Decal => Color::from_srgba_unmultiplied(24, 201, 186, 255),
            NodeFilter::InstakillBarrier => Color::from_srgba_unmultiplied(220, 60, 60, 255),
            NodeFilter::TurnbackBarrier => Color::from_srgba_unmultiplied(220, 120, 60, 255),
            NodeFilter::PlayerContainmentVolume => {
                Color::from_srgba_unmultiplied(192, 100, 192, 255)
            }
            NodeFilter::NamedArea => Color::from_srgba_unmultiplied(0, 127, 0, 255),
            NodeFilter::Unknown80809121 => Color::from_srgba_unmultiplied(96, 96, 255, 255),
            NodeFilter::Utility => Color::from_srgba_unmultiplied(119, 142, 160, 255),
            NodeFilter::Unknown => Color::from_srgba_unmultiplied(255, 255, 255, 255),
        }
    }
}

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
    Color::from_srgba_unmultiplied(r, g, b, 255)
}

impl EntityTag {
    pub fn color(&self) -> Color {
        match self {
            EntityTag::Havok => Color::from_srgba_unmultiplied(253, 185, 10, 255),
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

#[derive(Default, Component)]
pub struct Tags(pub FxHashSet<EntityTag>);

impl Tags {
    pub fn insert(&mut self, tag: EntityTag) {
        self.0.insert(tag);
    }

    // pub fn remove(&mut self, tag: EntityTag) {
    //     self.0.remove(&tag);
    // }
}

impl FromIterator<EntityTag> for Tags {
    fn from_iter<T: IntoIterator<Item = EntityTag>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

pub fn insert_tag(scene: &mut Scene, ent: Entity, tag: EntityTag) {
    if let Some(mut e) = scene.get_entity_mut(ent) {
        if let Some(mut tags) = e.get_mut::<Tags>() {
            tags.insert(tag);
            return;
        }

        scene.entity_mut(ent).insert_one(Tags::from_iter([tag]));
    }
}

pub fn remove_tag(scene: &mut Scene, ent: Entity, tag: EntityTag) {
    if let Some(mut e) = scene.get_entity_mut(ent) {
        if let Some(mut tags) = e.get_mut::<Tags>() {
            tags.0.remove(&tag);
        }
    }
}
