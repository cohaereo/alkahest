use crate::structure::{
    ExtendedHash, RelPointer, ResourcePointer, ResourcePointerWithClass, TablePointer, Tag,
};

use crate::types::{ResourceHash, Vector4};
use binrw::{BinRead, NullString};
use destiny_pkg::{TagHash, TagHash64};
use std::io::SeekFrom;

#[derive(Debug, BinRead)]
pub struct SActivity {
    pub file_size: u64,
    pub location_name: ResourceHash,
    pub unkc: ResourceHash,
    pub unk10: ResourceHash,
    pub unk14: ResourceHash,
    pub unk18: ResourcePointer,
    pub unk20: TagHash64,

    #[br(seek_before = SeekFrom::Start(0x40))]
    pub unk40: TablePointer<Unk80808926>,
    pub unk50: TablePointer<Unk80808924>,
    pub unk60: [u32; 4],
    pub unk70: ResourceHash,
    pub unk74: TagHash,
}

#[derive(Debug, BinRead)]
pub struct Unk80808e8b {
    pub file_size: u64,
    pub location_name: ResourceHash,
    pub unkc: u32,
    pub string_container: ExtendedHash,
    pub events: TagHash,
    pub patrols: TagHash,
    pub unk28: u32,
    pub unk2c: TagHash,
    pub tagbags: TablePointer<TagHash>, // 0x30
    pub unk40: u32,
    pub unk48: u32,
    pub activities: TablePointer<Unk8080892e>,
    pub destination_name: RelPointer<NullString>,
}

#[derive(Debug, BinRead)]
pub struct Unk8080892e {
    pub short_activity_name: ResourceHash,
    pub unk4: u32,
    pub unk8: ResourceHash,
    pub unkc: ResourceHash,
    pub activity_name: RelPointer<NullString>,
}

#[derive(Debug, BinRead)]
pub struct Unk80808924 {
    pub location_name: ResourceHash,
    pub activity_name: ResourceHash,
    pub bubble_name: ResourceHash,
    pub unkc: u32,
    pub unk10: ResourcePointer,
    pub unk18: TablePointer<Unk80808948>,
    pub map_references: TablePointer<ExtendedHash>,
}

#[derive(Debug, BinRead)]
pub struct Unk80808926 {
    pub location_name: ResourceHash,
    pub activity_name: ResourceHash,
    pub bubble_name: ResourceHash,
    pub unkc: ResourceHash,
    pub unk10: ResourceHash,
    pub unk14: u32,
    pub bubble_name2: ResourceHash,
    pub unk1c: u32,
    pub unk20: ResourceHash,
    pub unk24: ResourceHash,
    pub unk28: ResourceHash,
    pub unk2c: ResourceHash,
    pub unk30: ResourceHash,
    pub unk34: ResourceHash,
    pub unk38: ResourceHash,
    pub unk3c: u32,
    pub unk40: u32,
    pub unk44: u32,
    pub unk48: TablePointer<Unk80808948>,
    pub unk4c: [u32; 4],
}

#[derive(Debug, BinRead)]
pub struct Unk80808948 {
    pub location_name: ResourceHash,
    pub activity_name: ResourceHash,
    pub bubble_name: ResourceHash,
    pub activity_phase_name: ResourceHash,
    pub activity_phase_name2: ResourceHash,
    pub unk_entity_reference: Tag<Unk80808e89>,
}

#[derive(Debug, BinRead, Clone)]
pub struct Unk80808e89 {
    pub file_size: u64,
    pub unk8: u64,
    pub unk10: ResourcePointer,
    pub unk18: Tag<Unk80808ebe>,
    pub unk1c: u32,
    pub unk20: [u32; 4],
}

#[derive(Debug, BinRead, Clone)]
pub struct Unk80808ebe {
    pub file_size: u64,
    pub entity_resources: TablePointer<Tag<Unk80808943>>,
}

#[derive(Debug, BinRead, Clone)]
pub struct Unk80808943 {
    pub file_size: u64,
    #[br(seek_before = SeekFrom::Start(0x20))]
    pub entity_resource: TagHash,
}

#[derive(Debug, BinRead)]
pub struct SEntityResource {
    pub file_size: u64,
    pub unk8: ResourcePointer,
    pub unk10: ResourcePointerWithClass,
    pub unk18: ResourcePointerWithClass,

    #[br(seek_before = SeekFrom::Start(0x40))]
    pub resource_table1: TablePointer<()>,

    #[br(seek_before = SeekFrom::Start(0x60))]
    pub resource_table2: TablePointer<()>,

    #[br(seek_before = SeekFrom::Start(0x80))]
    pub unk80: TagHash,
    pub unk84: TagHash,
}

#[derive(Debug, BinRead)]
pub struct Unk808092d8 {
    pub unk0: [u32; 33],
    pub unk84: TagHash,
    pub unk88: u32,
    pub unk8c: u32,
    pub rotation: Vector4,
    pub translation: Vector4,
}

#[derive(Debug, BinRead)]
pub struct Unk80808cef {
    pub unk0: [u32; 22],
    pub unk58: TagHash,
}
