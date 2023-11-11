use std::io::SeekFrom;

use binrw::{binread, NullString};
use destiny_pkg::TagHash;

use crate::{
    structure::{ExtendedHash, RelPointer, TablePointer, Tag},
    types::Vector4,
};

#[binread]
#[derive(Debug)]
pub struct SRenderGlobals {
    pub file_size: u64,
    pub unk8: TablePointer<Unk8080870f>,
    pub unk18: TablePointer<()>,
}

#[binread]
#[derive(Debug)]
pub struct Unk8080870f {
    pub unk0: u32,
    pub unk4: u32,
    pub unk8: Tag<Unk808067a8>,
    pub unkc: u32,
}

#[binread]
#[derive(Debug)]
pub struct Unk808067a8 {
    pub file_size: u64,
    pub unk8: TagHash,
    _pad10: u32,
    pub scopes: TablePointer<Unk808067ad>,
    pub unk20: TablePointer<Unk808067ac>,
    pub unk30: TagHash,
    pub unk34: TagHash,
    pub unk38: TagHash,
}

#[binread]
#[derive(Debug)]
pub struct Unk808067ad {
    pub name: RelPointer<NullString>,
    pub unk8: u32,
    pub scope: Tag<SScope>,
}

#[binread]
#[derive(Debug)]
pub struct Unk808067ac {
    pub name: RelPointer<NullString>,
    pub unk8: u32,
    pub unkc: TagHash,
}

#[binread]
#[derive(Debug)]

pub struct SScope {
    pub file_size: u64,
    pub name: RelPointer<NullString>,

    #[br(seek_before(SeekFrom::Start(0x58)))]
    // TODO(cohae): Order *might* be incorrect
    pub stage_pixel: SScopeStage,
    pub stage_vertex: SScopeStage,
    pub stage_geometry: SScopeStage,
    pub stage_hull: SScopeStage,
    pub stage_compute: SScopeStage,
    pub stage_domain: SScopeStage,
}

#[binread]
#[derive(Debug)]

pub struct SScopeStage {
    pub unk0: u64,
    pub bytecode: TablePointer<u8>,
    pub bytecode_constants: TablePointer<Vector4>,
    pub samplers: TablePointer<ExtendedHash>,
    pub unk38: TablePointer<Vector4>,
    pub unk48: [u32; 4],

    pub constant_buffer_slot: u32,
    pub constant_buffer: TagHash,

    pub unksomething: [u32; 10],
}
