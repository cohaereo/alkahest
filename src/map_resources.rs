use crate::ecs::transform::Transform;
use crate::icons::{
    ICON_ACCOUNT_CONVERT, ICON_ALERT, ICON_CHESS_PAWN, ICON_DROPBOX, ICON_FLARE, ICON_HELP,
    ICON_HELP_BOX_OUTLINE, ICON_LIGHTBULB_ON, ICON_PINE_TREE, ICON_REPLY, ICON_SKULL, ICON_SPHERE,
    ICON_SPOTLIGHT_BEAM, ICON_STICKER, ICON_TAG, ICON_VOLUME_HIGH, ICON_WAVES,
};
use crate::map::{Unk80806b7f, Unk80809178, Unk80809802};
use crate::render::debug::{CustomDebugShape, DebugShapes};
use crate::structure::ExtendedHash;
use crate::structure::ResourcePointer;
use crate::types::AABB;
use destiny_pkg::TagHash;
use glam::{Mat4, Vec3};
use itertools::Itertools;

use strum::{EnumCount, EnumIs, EnumVariantNames};

#[derive(Clone, EnumVariantNames, EnumCount, EnumIs)]
#[repr(u8)]
#[strum(serialize_all = "snake_case")]
pub enum MapResource {
    /// Generic data entry with no resource
    Entity(ExtendedHash, u64),
    Decal {
        material: TagHash,
        bounds: AABB,
        scale: f32,
    },
    CubemapVolume(Box<Unk80806b7f>, AABB),
    RespawnPoint(u32),
    AmbientSound(Option<Unk80809802>),
    Light(AABB, TagHash, usize),
    ShadowingLight(TagHash),
    NamedArea(Unk80809178, String, Option<CustomDebugShape>),

    Unknown(u32, u64, ExtendedHash, ResourcePointer, TagHash),
    Unk808067b5(TagHash),
    Unk80806aa3(AABB, TagHash, Mat4),
    Unk808085c0,
    Unk80806a40,
    Decoration(AABB, TagHash),
    InstantKillBarrier(TagHash, u32, Option<CustomDebugShape>),
    TurnbackKillBarrier(TagHash, u32, Option<CustomDebugShape>),
    Unk80809121(TagHash),
    Unk808068d4(TagHash),
    PlayAreaBounds(TagHash, Option<CustomDebugShape>),
    Unk80808246(TagHash, u32, Option<CustomDebugShape>),
}

impl MapResource {
    pub fn debug_string(&self) -> String {
        match self {
            MapResource::Entity(hash, world_id) => {
                let hash32 = if let Some(h32) = hash.hash32() {
                    format!(" ({h32})")
                } else {
                    String::new()
                };
                format!("Entity {hash:?}{hash32}\n(0x{world_id:016x})",)
            }
            MapResource::Decal { material, .. } => {
                format!("Decal (mat {material})")
            }
            MapResource::Unknown(u, world_id, entity, res_ptr, table_tag) => {
                let hash32 = if let Some(h32) = entity.hash32() {
                    format!("\nEntity {h32}")
                } else {
                    String::new()
                };
                format!(
                    "Unknown {:08X} (0x{world_id:016x})\nResource table {} @ 0x{:x}{hash32}",
                    u.to_be(),
                    table_tag,
                    res_ptr.offset,
                )
            }
            MapResource::Unk808067b5 { .. } => "Unk808067b5 (light flare)".to_string(),
            MapResource::CubemapVolume(c, _aabb) => {
                format!(
                    "Cubemap Volume\n'{}' (cube={}, volume={})",
                    *c.cubemap_name, c.cubemap_texture, c.unk1c0
                )
            }
            MapResource::Unk80806aa3(_, t, _) => format!("Unk80806aa3 (model {t})"),
            MapResource::Light(_, t, i) => format!("Light ({t}+{i})"),
            MapResource::RespawnPoint(v) => format!("Respawn Point (0x{v:X})"),
            MapResource::Unk808085c0 => "Unk808085c0".to_string(),
            MapResource::Unk80806a40 => "Unk80806d19".to_string(),
            MapResource::InstantKillBarrier(h, i, s) => {
                if s.is_some() {
                    format!("InstantKillBarrier (havok {h}:{i})")
                } else {
                    format!(
                        "InstantKillBarrier (havok {h}:{i})\n{} Shape visualization failed to load",
                        ICON_ALERT
                    )
                }
            }
            MapResource::TurnbackKillBarrier(h, i, s) => {
                if s.is_some() {
                    format!("TurnbackKillBarrier (havok {h}:{i})")
                } else {
                    format!(
                        "TurnbackKillBarrier (havok {h}:{i})\n{} Shape visualization failed to load",
                        ICON_ALERT
                    )
                }
            }
            MapResource::Unk80809121(h) => format!("Unk80809121 (havok {h})"),
            MapResource::AmbientSound(s) => {
                if let Some(s) = s {
                    format!(
                        "Ambient Sound\n(streams [{}])",
                        s.streams.iter().map(|t| t.to_string()).join(", ")
                    )
                } else {
                    "Ambient Sound (no header?)".to_string()
                }
            }
            MapResource::Decoration(_, t) => format!("Decoration ({t})"),
            MapResource::ShadowingLight(t) => format!("Shadowing Light ({t})"),
            MapResource::NamedArea(_, s, shape) => {
                if shape.is_some() {
                    format!("NamedArea ({s})")
                } else {
                    format!(
                        "NamedArea ({s})\n{} Shape visualization failed to load",
                        ICON_ALERT
                    )
                }
            }
            MapResource::Unk808068d4(e) => format!("Unk808068d4 ({e}) (water)"),
            MapResource::PlayAreaBounds(t, s) => {
                if s.is_some() {
                    format!("Play Area Bounds (havok {t})")
                } else {
                    format!(
                        "Play Area Bounds (havok {t})\n{} Havok shape visualization failed to load",
                        ICON_ALERT
                    )
                }
            }
            MapResource::Unk80808246(t, i, s) => {
                if s.is_some() {
                    format!("Unk80808246 (havok {t}:{i})")
                } else {
                    format!(
                        "Unk80808246 (havok {t}:{i})\n{} Havok shape visualization failed to load",
                        ICON_ALERT
                    )
                }
            }
        }
    }

    pub fn draw_debug_shape(&self, transform: &Transform, debug_shapes: &mut DebugShapes) {
        match self {
            MapResource::Decal { scale, .. } => debug_shapes.cube_extents(
                transform.translation,
                Vec3::splat(*scale / 2.0),
                transform.rotation,
                darken_color(self.debug_color()),
                false,
            ),
            MapResource::CubemapVolume(_, bounds) => debug_shapes.cube_aabb(
                *bounds,
                transform.rotation,
                darken_color(self.debug_color()),
                true,
            ),
            MapResource::Decoration(bounds, _) => debug_shapes.cube_aabb(
                *bounds,
                transform.rotation,
                darken_color(self.debug_color()),
                false,
            ),
            MapResource::ShadowingLight(_) => debug_shapes.line_orientation(
                transform.translation,
                transform.rotation,
                2.5,
                self.debug_color(),
            ),
            MapResource::Light(_bounds, _, _) => {
                debug_shapes.line_orientation(
                    transform.translation,
                    transform.rotation,
                    1.0,
                    self.debug_color(),
                );
                // debug_shapes.cube_aabb(
                //     *bounds,
                //     Quat::IDENTITY,
                //     darken_color(self.debug_color()),
                //     false,
                // )
            }
            MapResource::RespawnPoint(_) => debug_shapes.line_orientation(
                transform.translation,
                transform.rotation,
                1.0,
                self.debug_color(),
            ),
            MapResource::InstantKillBarrier(_, _, Some(shape)) => {
                debug_shapes.custom_shape(*transform, shape.clone(), self.debug_color(), true);
            }
            MapResource::TurnbackKillBarrier(_, _, Some(shape)) => {
                debug_shapes.custom_shape(*transform, shape.clone(), self.debug_color(), true);
            }
            MapResource::PlayAreaBounds(_, Some(shape)) => {
                debug_shapes.custom_shape(*transform, shape.clone(), self.debug_color(), true);
            }
            MapResource::NamedArea(_, _, Some(shape)) => {
                debug_shapes.custom_shape(*transform, shape.clone(), self.debug_color(), true);
            }
            MapResource::Unk80808246(_, _, Some(shape)) => {
                debug_shapes.custom_shape(*transform, shape.clone(), self.debug_color(), true);
            }
            _ => {}
        }
    }

    pub fn debug_color(&self) -> [u8; 3] {
        const RANDOM_COLORS: [[u8; 3]; 16] = [
            [0xFF, 0x00, 0x00],
            [0x00, 0xFF, 0x00],
            [0x00, 0x00, 0xFF],
            [0xFF, 0xFF, 0x00],
            [0xFF, 0x00, 0xFF],
            [0x00, 0xFF, 0xFF],
            [0x00, 0x00, 0x00],
            [0x80, 0x00, 0x00],
            [0x00, 0x80, 0x00],
            [0x00, 0x00, 0x80],
            [0x80, 0x80, 0x00],
            [0x80, 0x00, 0x80],
            [0x00, 0x80, 0x80],
            [0x80, 0xFF, 0x80],
            [0xC0, 0x00, 0x00],
            [0x00, 0xC0, 0x00],
        ];

        match self {
            MapResource::Unknown(u, _, _, _, _) => RANDOM_COLORS[*u as usize % 16],
            _ => Self::debug_color_from_index(self.index()),
        }
    }

    pub fn debug_icon(&self) -> char {
        Self::debug_icon_from_index(self.index())
    }

    pub fn debug_id(&self) -> &'static str {
        Self::index_to_id(self.index())
    }
}

fn darken_color(v: [u8; 3]) -> [u8; 3] {
    [
        (v[0] as f32 * 0.75) as u8,
        (v[1] as f32 * 0.75) as u8,
        (v[2] as f32 * 0.75) as u8,
    ]
}

macro_rules! mapresource_info {
    ($($id:literal, $name:ident, $color:expr, $icon:expr)*) => {
        impl MapResource {
            pub fn debug_color_from_index(index: usize) -> [u8; 3] {
                match index {
                    $(
                        $id => $color,
                    )*
                    _ => [0xFF, 0xFF, 0xFF],
                }
            }

            pub fn debug_icon_from_index(index: usize) -> char {
                match index {
                    $(
                        $id => $icon,
                    )*
                    _ => ICON_HELP_BOX_OUTLINE,
                }
            }

            pub fn index_to_id(index: usize) -> &'static str {
                match index {
                    $(
                        $id => stringify!($name),
                    )*
                    _ => "InvalidResource",
                }
            }

            pub fn id_to_index(id: &str) -> Option<usize> {
                match id {
                    $(
                        stringify!($name) => Some($id),
                    )*
                    _ => None,
                }
            }

            pub fn index(&self) -> usize {
                match self {
                    $(
                        Self::$name { .. } => $id,
                    )*
                }
            }

            // Ugly, but gets optimized away to whatever is the highest value thanks to const functions
            pub fn max_index() -> usize {
                let mut max = 0;
                $(
                    max = max.max($id);
                )*

                max
            }
        }
    };
}

mapresource_info!(
    0, Entity, [255, 255, 255], ICON_CHESS_PAWN
    1, Decal, [50, 255, 255], ICON_STICKER
    2, RespawnPoint, [220, 20, 20], ICON_ACCOUNT_CONVERT
    3, AmbientSound, [0, 192, 0], ICON_VOLUME_HIGH
    4, ShadowingLight, [255, 255, 0], ICON_SPOTLIGHT_BEAM
    5, Light, [255, 255, 0], ICON_LIGHTBULB_ON
    6, Unknown, [255, 255, 255], ICON_HELP
    7, Unk808067b5, [220, 220, 20], ICON_FLARE
    8, Unk80806aa3, [96, 96, 255], ICON_HELP
    9, Unk808085c0, [255, 96, 96], ICON_HELP
    10, Unk80806a40, [255, 44, 44], ICON_HELP
    11, Decoration, [80, 210, 80], ICON_PINE_TREE
    12, Unk80809121, [96, 96, 255], ICON_HELP
    13, Unk808068d4, [22, 230, 190], ICON_WAVES
    14, CubemapVolume, [50, 255, 50], ICON_SPHERE
    15, NamedArea, [0, 127, 0], ICON_TAG
    16, InstantKillBarrier, [220, 60, 60], ICON_SKULL
    17, TurnbackKillBarrier, [220, 120, 60], ICON_REPLY
    18, PlayAreaBounds, [192, 100, 192], ICON_DROPBOX
    19, Unk80808246, [220, 60, 60], ICON_HELP
);
