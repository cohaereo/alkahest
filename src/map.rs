use crate::structure::{RelPointer, ResourcePointer, TablePointer};
use crate::types::{DestinyHash, TagHash64, Vector4};
use binrw::BinRead;
use destiny_pkg::TagHash;
use std::io::SeekFrom;

// D2Class_1E898080
#[derive(BinRead, Debug)]
pub struct Unk80807dae {
    pub file_size: u64,
    // 808091e0
    pub child_map_reference: TagHash,
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
    pub map_resources: TablePointer<Unk808084c1>,
}

// TODO: Custom reader once new tag parser comes around
#[derive(BinRead, Debug)]
pub struct Unk808084c1 {
    // 80808a54
    pub hash32: TagHash,
    pub is_hash32: u32,
    pub hash64: TagHash64, // 80808a54
}

// D2Class_07878080
#[derive(BinRead, Debug)]
pub struct Unk80808a54 {
    pub file_size: u64,
    #[br(seek_before(SeekFrom::Start(0x28)))]
    pub data_tables: TablePointer<TagHash>, // 808099d6
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
    // 80809c0f ?
    pub unk0: TagHash,
    pub unk4: [u32; 3],
    pub rotation: Vector4,
    pub translation: Vector4,
    pub unk30: [u32; 11],
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
    // 8080966d
    pub placement_group: TagHash,
    pub unkc: [u32; 7],
}

// Terrain resource
#[derive(BinRead, Debug)]
pub struct Unk8080714b {
    #[br(seek_before(SeekFrom::Current(0x10)))]
    pub unk10: u16,
    pub unk12: u16,
    pub unk14: DestinyHash,
    pub terrain: TagHash,
    pub terrain_bounds: TagHash,
}

/// Terrain
#[derive(BinRead, Debug)]
pub struct Unk8080714f {
    pub file_size: u64,
    #[br(seek_before(SeekFrom::Start(0x10)))]
    pub unk10: Vector4,
    pub unk20: Vector4,
    pub unk30: Vector4,
    #[br(seek_before(SeekFrom::Start(0x58)))]
    pub mesh_groups: TablePointer<Unk80807154>,

    pub vertex_buffer: TagHash,
    pub vertex2_buffer: TagHash,
    pub indices: TagHash,
    pub material1: TagHash,
    pub material2: TagHash,

    #[br(seek_before(SeekFrom::Start(0x80)))]
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
