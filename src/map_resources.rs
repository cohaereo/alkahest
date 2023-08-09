use crate::icons::{
    ICON_CHESS_PAWN, ICON_HELP, ICON_HELP_CIRCLE, ICON_LIGHTBULB_ON, ICON_PANORAMA, ICON_SPHERE,
};
use crate::structure::RelPointer;
use crate::types::{DestinyHash, Vector4};
use binrw::{BinRead, NullString};
use destiny_pkg::TagHash;
use std::io::SeekFrom;

#[derive(Clone)]
pub enum MapResource {
    // PlacementGroup(TagHash),
    // Terrain(Unk8080714b),
    /// Generic data entry with no resource
    Entity(TagHash),
    CubemapVolume(Unk80806b7f),
    PointLight(TagHash),
    Unknown(u32),
}

impl MapResource {
    pub fn debug_string(&self) -> String {
        match self {
            MapResource::Entity(e) => format!("Entity 0x{:08x}", e.0),
            MapResource::CubemapVolume(c) => {
                format!("Cubemap Volume\n'{}'", c.cubemap_name.to_string())
            }
            MapResource::PointLight(_) => format!("Point light"),
            MapResource::Unknown(u) => format!("Unknown {u:08x}"),
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
            [0x80, 0x80, 0x80],
            [0xC0, 0x00, 0x00],
            [0x00, 0xC0, 0x00],
        ];

        match self {
            MapResource::Entity(_) => [255, 255, 255],
            MapResource::CubemapVolume(_) => [50, 255, 50],
            MapResource::PointLight(_) => [220, 220, 20],
            MapResource::Unknown(u) => RANDOM_COLORS[*u as usize % 16],
        }
    }

    pub fn debug_icon(&self) -> char {
        match self {
            MapResource::Entity(_) => ICON_CHESS_PAWN,
            MapResource::CubemapVolume(_) => ICON_SPHERE,
            MapResource::PointLight(_) => ICON_LIGHTBULB_ON,
            MapResource::Unknown(_) => ICON_HELP,
        }
    }
}

/// Terrain resource
#[derive(BinRead, Debug, Clone)]
pub struct Unk8080714b {
    #[br(seek_before(SeekFrom::Current(0x10)))]
    pub unk10: u16,
    pub unk12: u16,
    pub unk14: DestinyHash,
    pub terrain: TagHash,
    pub terrain_bounds: TagHash,
}

/// Cubemap volume resource
#[derive(BinRead, Debug, Clone)]
pub struct Unk80806b7f {
    #[br(seek_before(SeekFrom::Current(0x20)))]
    pub unk20: Vector4,
    pub unk30: Vector4,
    pub unk40: f32,
    pub unk44: [u32; 3],
    pub unk50: Vector4,
    pub unk60: Vector4,

    pub unk70: [u32; 20],

    // Transform matrices?
    pub unkc0: [Vector4; 4],
    pub unk100: [Vector4; 4],

    pub unk140: [u32; 20],

    pub cubemap_name: RelPointer<NullString>,
    pub cubemap_texture: TagHash,
    pub unk19c: u32,
    pub unk1a0: TagHash,
    pub unk1a4: [u32; 7],
}
