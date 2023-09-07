use crate::icons::{
    ICON_ACCOUNT_CONVERT, ICON_CHESS_PAWN, ICON_HELP, ICON_HELP_BOX_OUTLINE, ICON_LIGHTBULB_ON,
    ICON_SPHERE, ICON_STICKER, ICON_VOLUME_HIGH,
};
use crate::render::debug::DebugShapes;
use crate::structure::{RelPointer, TablePointer};
use crate::types::{DestinyHash, Vector4, AABB};
use binrw::{BinRead, NullString};
use destiny_pkg::TagHash;
use glam::{Quat, Vec3, Vec4, Vec4Swizzles};
use itertools::Itertools;
use std::io::SeekFrom;
use strum::{EnumCount, EnumIs, EnumVariantNames};

#[derive(Clone, EnumVariantNames, EnumCount, EnumIs)]
#[repr(u8)]
pub enum MapResource {
    // PlacementGroup(TagHash),
    // Terrain(Unk8080714b),
    /// Generic data entry with no resource
    Entity(TagHash) = 0,
    CubemapVolume(Box<Unk80806b7f>, AABB) = 1,
    PointLight(TagHash) = 2,
    Decal {
        material: TagHash,
        scale: f32,
    } = 3,
    Unknown(u32) = 4,
    Unk80806df1 = 5,
    Unk80806f38 = 6,
    RespawnPoint = 7,
    AmbientSound(Unk80809802) = 8,
    Unk808071ad(AABB) = 9,
}

impl MapResource {
    pub fn debug_string(&self) -> String {
        match self {
            MapResource::Entity(e) => format!("Entity {:08X}", e.0.to_be()),
            MapResource::CubemapVolume(c, aabb) => {
                format!(
                    "Cubemap Volume ({:.0}m³)\n'{}' ({:08X})",
                    aabb.volume(),
                    *c.cubemap_name,
                    c.cubemap_texture.0.to_be()
                )
            }
            MapResource::Decal { material, scale } => {
                format!("Decal (mat {material}, scale {scale})")
            }
            MapResource::PointLight { .. } => "Point light".to_string(),
            MapResource::Unknown(u) => format!("Unknown {:08X}", u.to_be()),
            MapResource::Unk80806df1 => "Unk80806df1".to_string(),
            MapResource::Unk80806f38 => "Unk80806f38".to_string(),
            MapResource::RespawnPoint => "Respawn Point".to_string(),
            MapResource::AmbientSound(s) => format!(
                "Ambient Sound (bank {})\n(streams [{}])",
                s.soundbank,
                s.streams.iter().map(|t| t.to_string()).join(", ")
            ),
            MapResource::Unk808071ad(volume) => {
                format!("Unk808071ad (volume {}m³)", volume.volume())
            }
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
            MapResource::Entity { .. } => [255, 255, 255],
            MapResource::CubemapVolume(..) => [50, 255, 50],
            MapResource::PointLight { .. } => [220, 220, 20],
            MapResource::Decal { .. } => [50, 255, 255],
            MapResource::Unknown(u) => RANDOM_COLORS[*u as usize % 16],
            MapResource::Unk80806df1 => RANDOM_COLORS[0x80806df1 % 16],
            MapResource::Unk80806f38 => RANDOM_COLORS[0x80806f38 % 16],
            MapResource::RespawnPoint => [220, 20, 20],
            MapResource::AmbientSound { .. } => RANDOM_COLORS[0x80806b5b % 16],
            MapResource::Unk808071ad { .. } => RANDOM_COLORS[0x808071ad % 16],
        }
    }

    pub fn debug_icon(&self) -> char {
        match self {
            MapResource::Entity { .. } => ICON_CHESS_PAWN,
            MapResource::CubemapVolume(..) => ICON_SPHERE,
            MapResource::PointLight { .. } => ICON_LIGHTBULB_ON,
            MapResource::Decal { .. } => ICON_STICKER,
            MapResource::Unknown { .. } => ICON_HELP,
            MapResource::RespawnPoint => ICON_ACCOUNT_CONVERT,
            MapResource::Unk80806df1
            | MapResource::Unk80806f38
            | MapResource::Unk808071ad { .. } => ICON_HELP_BOX_OUTLINE,
            MapResource::AmbientSound { .. } => ICON_VOLUME_HIGH,
        }
    }

    pub fn draw_debug_shape(
        &self,
        translation: Vec4,
        rotation: Quat,
        debug_shapes: &mut DebugShapes,
    ) {
        match self {
            MapResource::Decal { scale, .. } => debug_shapes.cube_extents(
                translation.xyz(),
                Vec3::splat(*scale),
                rotation,
                darken_color(self.debug_color()),
                false,
            ),
            MapResource::CubemapVolume(_, bounds) => {
                debug_shapes.cube_aabb(*bounds, rotation, darken_color(self.debug_color()), true)
            }
            MapResource::Unk808071ad(bounds) => {
                debug_shapes.cube_aabb(*bounds, rotation, darken_color(self.debug_color()), true)
            }
            _ => {}
        }
    }

    // TODO(cohae): Make this easier to work with
    pub fn get_icon_by_index(i: u8) -> char {
        match i {
            0 => ICON_CHESS_PAWN,
            1 => ICON_SPHERE,
            2 => ICON_LIGHTBULB_ON,
            3 => ICON_STICKER,
            4 => ICON_HELP,
            7 => ICON_ACCOUNT_CONVERT,
            8 => ICON_VOLUME_HIGH,
            _ => ICON_HELP_BOX_OUTLINE,
        }
    }

    pub fn index(&self) -> u8 {
        unsafe { (self as *const MapResource as *const u8).read() }
    }
}

fn darken_color(v: [u8; 3]) -> [u8; 3] {
    [
        (v[0] as f32 * 0.75) as u8,
        (v[1] as f32 * 0.75) as u8,
        (v[2] as f32 * 0.75) as u8,
    ]
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
    pub cubemap_extents: Vector4,
    /// Represents the visual center of the cubemap
    pub cubemap_center: Vector4,
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

/// Decal collection resource
#[derive(BinRead, Debug, Clone)]
pub struct Unk80806e68 {
    pub file_size: u64,
    pub instances: TablePointer<Unk80806e6c>,
    pub transforms: TablePointer<Vector4>, // 80806e6d
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80806e6c {
    pub material: TagHash,
    pub start: u16,
    pub count: u16,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80806df3 {
    pub file_size: u64,
    pub unk8: TablePointer<Unk80806dec>,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80806dec {
    pub material: TagHash,
    pub index_buffer: TagHash,
    pub vertex_buffer: TagHash,
    pub unkc: u32,
    pub unk10: [u32; 4],

    pub translation: Vector4,

    pub unk30: Vector4,
    pub unk40: Vector4,
    pub unk50: Vector4,
}

// Unknown resource (some kind of octree?)
#[derive(BinRead, Debug, Clone)]
pub struct Unk80807268 {
    pub file_size: u64,
    /// Vertex buffer
    pub unk8: TagHash,
    pub unkc: u32,
    pub unk10: TablePointer<Unk8080726a>,
    pub unk20: [u32; 6],
    /// Vertex buffer
    pub unk38: TagHash,
    pub unk3c: u32,
    pub unk40: TablePointer<Unk8080726a>,
    pub unk50: TablePointer<Unk8080726d>,
    pub unk60: TablePointer<u16>,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk8080726a {
    pub unk0: [u32; 4],
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk8080726d {
    pub unk0: Vector4,
    pub unk10: Vector4,
    pub unk20: Vector4,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80809162 {
    pub file_size: u64,
    pub unk8: TablePointer<Unk80809164>,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80809164 {
    pub unk0: Vector4,
    pub unk10: Vector4,
    pub unk20: [u32; 4],
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80809802 {
    pub file_size: u64,
    pub unk8: TagHash,
    pub unkc: TagHash,
    pub unk10: TagHash,
    pub soundbank: TagHash,
    pub streams: TablePointer<TagHash>,
    pub unk28: TagHash,
}
