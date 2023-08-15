use crate::icons::{ICON_CHESS_PAWN, ICON_HELP, ICON_LIGHTBULB_ON, ICON_SPHERE, ICON_STICKER};
use crate::structure::{RelPointer, TablePointer};
use crate::types::{DestinyHash, Vector4};
use binrw::{BinRead, NullString};
use destiny_pkg::TagHash;
use std::io::SeekFrom;
use std::mem::MaybeUninit;
use strum::{EnumCount, EnumIs, EnumVariantNames};

#[derive(Clone, EnumVariantNames, EnumCount, EnumIs)]
#[repr(u8)]
pub enum MapResource {
    // PlacementGroup(TagHash),
    // Terrain(Unk8080714b),
    /// Generic data entry with no resource
    Entity(TagHash),
    CubemapVolume(Unk80806b7f),
    PointLight(TagHash),
    Decal {
        material: TagHash,
    },
    Unknown(u32),
}

impl MapResource {
    pub fn debug_string(&self) -> String {
        match self {
            MapResource::Entity(e) => format!("Entity {:08X}", e.0.to_be()),
            MapResource::CubemapVolume(c) => {
                format!(
                    "Cubemap Volume\n'{}' ({:08X})",
                    c.cubemap_name.to_string(),
                    c.cubemap_texture.0.to_be()
                )
            }
            MapResource::Decal { material } => format!("Decal (mat {material})"),
            MapResource::PointLight(_) => format!("Point light"),
            MapResource::Unknown(u) => format!("Unknown {:08X}", u.to_be()),
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
            MapResource::Decal { .. } => [50, 255, 255],
            MapResource::Unknown(u) => RANDOM_COLORS[*u as usize % 16],
        }
    }

    pub fn debug_icon(&self) -> char {
        match self {
            MapResource::Entity(_) => ICON_CHESS_PAWN,
            MapResource::CubemapVolume(_) => ICON_SPHERE,
            MapResource::PointLight(_) => ICON_LIGHTBULB_ON,
            MapResource::Decal { .. } => ICON_STICKER,
            MapResource::Unknown(_) => ICON_HELP,
        }
    }

    /// Creates a dud variant instance used for obtaining color and icon
    unsafe fn get_by_index(i: u8) -> MapResource {
        let e = (i, 0u32);
        let mut mm: MaybeUninit<MapResource> = MaybeUninit::zeroed();
        mm.as_mut_ptr().copy_from(&e as *const (u8, u32) as _, 1);
        mm.assume_init()
    }

    // pub fn get_color_by_index(i: u8) -> [u8; 3] {
    //     unsafe { Self::get_by_index(i) }.debug_color()
    // }

    pub fn get_icon_by_index(i: u8) -> char {
        unsafe { Self::get_by_index(i) }.debug_icon()
    }

    pub fn index(&self) -> u8 {
        unsafe { (self as *const MapResource as *const u8).read() }
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
