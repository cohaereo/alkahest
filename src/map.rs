use crate::overlays::resource_nametags::ResourcePoint;
use crate::packages::package_manager;
use crate::render::scopes::ScopeRigidModel;
use crate::render::ConstantBuffer;
use crate::statics::Unk8080966d;
use crate::structure::{ResourcePointer, TablePointer, Tag};
use crate::types::{DestinyHash, Vector4};
use binrw::{BinRead, BinReaderExt};
use destiny_pkg::{TagHash, TagHash64};

use std::io::SeekFrom;

// D2Class_1E898080
#[derive(BinRead, Debug)]
pub struct Unk80807dae {
    pub file_size: u64,
    // 808091e0
    pub child_map: Tag<Unk808091e0>,
    pub unkc: u32,

    pub unk10: u64,
    pub map_name: DestinyHash,

    #[br(seek_before(SeekFrom::Start(0x40)))]
    pub unk40: TablePointer<Unk80809644>,
}

#[derive(BinRead, Debug)]
pub struct Unk80809644 {
    pub unk0: u32,
    pub unk4: u32,
    pub unk8: u32,
    pub unkc: u32, // 8080964e
}

// D2Class_01878080
#[derive(BinRead, Debug)]
pub struct Unk808091e0 {
    pub file_size: u64,
    pub map_resources: TablePointer<ExtendedHash>,
}

// TODO: Custom reader once new tag parser comes around
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum ExtendedHash {
    Hash32(TagHash),
    Hash64(TagHash64),
}

impl ExtendedHash {
    /// Key that is safe to use for caching/lookup tables
    pub fn key(&self) -> u64 {
        match self {
            ExtendedHash::Hash32(v) => v.0 as u64,
            ExtendedHash::Hash64(v) => v.0,
        }
    }

    /// Will lookup hash64 in package managers's h64 table in the case of a 64 bit hash
    pub fn hash32(&self) -> Option<TagHash> {
        match self {
            ExtendedHash::Hash32(v) => Some(*v),
            ExtendedHash::Hash64(v) => package_manager().hash64_table.get(&v.0).map(|v| v.hash32),
        }
    }
}

impl BinRead for ExtendedHash {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let hash32: TagHash = reader.read_type(endian)?;
        let is_hash32: u32 = reader.read_type(endian)?;
        let hash64: TagHash64 = reader.read_type(endian)?;

        if is_hash32 != 0 {
            Ok(ExtendedHash::Hash32(hash32))
        } else {
            Ok(ExtendedHash::Hash64(hash64))
        }
    }
}

// D2Class_07878080
#[derive(BinRead, Debug)]
pub struct Unk80808a54 {
    pub file_size: u64,
    #[br(seek_before(SeekFrom::Start(0x28)))]
    pub data_tables: TablePointer<Tag<Unk808099d6>>,
}

// D2Class_83988080
#[derive(BinRead, Debug)]
pub struct Unk808099d6 {
    pub file_size: u64,
    pub data_entries: TablePointer<Unk808099d8>,
}

// D2Class_85988080
#[derive(BinRead, Debug)]
pub struct Unk808099d8 {
    // 80809c0f
    pub rotation: Vector4,    // 0x0
    pub translation: Vector4, // 0x10
    pub entity: TagHash,      // 0x20
    pub unk24: u32,
    pub entity2: TagHash,
    pub unk2c: u32,
    pub unk30: [u32; 11], //
    pub unk5c: f32,
    pub unk60: u32,
    pub unk64: DestinyHash,
    pub unk68: [u32; 4],
    pub data_resource: ResourcePointer,
    pub unk80: [u32; 4],
}

#[derive(BinRead, Debug)]
pub struct Unk80806ef4 {
    pub unk0: u64,
    pub placement_group: Tag<Unk8080966d>,
    pub unkc: [u32; 7],
}

/// Terrain
#[derive(BinRead, Debug)]
pub struct Unk8080714f {
    pub file_size: u64,
    #[br(seek_before(SeekFrom::Start(0x10)))]
    pub unk10: Vector4,
    pub unk20: Vector4,
    pub unk30: Vector4,
    #[br(seek_before(SeekFrom::Start(0x50)))]
    pub mesh_groups: TablePointer<Unk80807154>,

    pub vertex_buffer: TagHash,
    pub vertex_buffer2: TagHash,
    pub indices: TagHash,
    pub material1: TagHash,
    pub material2: TagHash,

    #[br(seek_before(SeekFrom::Start(0x78)))]
    pub mesh_parts: TablePointer<Unk80807152>,
}

#[derive(BinRead, Debug)]
pub struct Unk80807154 {
    pub unk0: f32,
    pub unk4: f32,
    pub unk8: f32,
    pub unkc: f32,
    pub unk10: f32,
    pub unk14: f32,
    pub unk18: f32,
    pub unk1c: u32,
    pub unk20: Vector4,
    pub unk30: u32,
    pub unk34: u32,
    pub unk38: u32,
    pub unk3c: u32,
    pub unk40: u32,
    pub unk44: u32,
    pub unk48: u32,
    pub unk4c: u32,
    pub dyemap: TagHash,
    pub unk54: u32,
    pub unk58: u32,
    pub unk5c: u32,
}

#[derive(BinRead, Debug)]
pub struct Unk80807152 {
    pub material: TagHash,
    pub index_start: u32,
    pub index_count: u16,
    pub group_index: u8,
    pub detail_level: u8,
}

pub struct MapData {
    pub hash: TagHash,
    pub name: String,
    pub placement_groups: Vec<Tag<Unk8080966d>>,
    pub resource_points: Vec<(ResourcePoint, ConstantBuffer<ScopeRigidModel>)>,
    pub terrains: Vec<TagHash>,
}

pub struct MapDataList {
    pub current_map: usize, // TODO(cohae): Shouldn't be here
    pub maps: Vec<MapData>,
}

impl MapDataList {
    pub fn current_map(&self) -> Option<&MapData> {
        self.maps.get(self.current_map % self.maps.len())
    }
}

#[derive(BinRead, Debug)]
pub struct Unk80807164 {
    pub file_size: u64,
    pub unk8: TablePointer<TagHash>,
    pub unk18: TablePointer<u32>,
    pub unk28: TablePointer<u32>,
    pub unk38: TablePointer<u32>,
    pub unk48: TagHash,
    pub unk4c: TagHash,
    pub unk50: TablePointer<u32>,
    pub unk60: [u32; 4],
    pub unk70: Vector4,
    pub unk80: Vector4,
}
