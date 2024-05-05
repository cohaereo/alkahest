use destiny_pkg::TagHash;
use glam::Vec4;
use tiger_parse::{tiger_tag, NullString, Pointer};

use super::{Tag, WideHash};

#[derive(Debug)]
#[tiger_tag(id = 0x8080978C)]
pub struct SRenderGlobals {
    pub file_size: u64,
    pub unk8: Vec<SUnk8080870f>,
    pub unk18: Vec<()>,
}

#[derive(Debug)]
#[tiger_tag(id = 0x8080870F)]
pub struct SUnk8080870f {
    pub unk0: u32,
    pub unk4: u32,
    pub unk8: Tag<SUnk808067a8>,
    pub unkc: u32,
}

#[derive(Debug)]
#[tiger_tag(id = 0x808067a8)]
pub struct SUnk808067a8 {
    pub file_size: u64,
    pub unk8: TagHash,
    _pad10: u32,
    pub scopes: Vec<SUnk808067ad>,
    pub unk20: Vec<SUnk808067ac>,
    /// Lookup textures
    pub unk30: Tag<SUnk808066ae>,
    pub unk34: Tag<SUnk8080822d>,
    pub unk38: TagHash,
}

#[derive(Debug)]
#[tiger_tag(id = 0x808066ae)]
pub struct SUnk808066ae {
    pub file_size: u64,
    pub specular_tint_lookup_texture: TagHash,
    pub specular_lobe_lookup_texture: TagHash,
    pub specular_lobe_3d_lookup_texture: TagHash,
    pub iridescence_lookup_texture: TagHash,
}

#[derive(Debug)]
#[tiger_tag(id = 0x808067AD)]
pub struct SUnk808067ad {
    pub name: Pointer<NullString>,
    pub unk8: u32,
    pub scope: Tag<SScope>,
}

#[derive(Debug)]
#[tiger_tag(id = 0x808067AC)]
pub struct SUnk808067ac {
    pub name: Pointer<NullString>,
    pub unk8: u32,
    pub technique: TagHash,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x80806DBA, size = 0x400)]

pub struct SScope {
    pub file_size: u64,
    pub name: Pointer<NullString>,

    #[tag(offset = 0x48)]
    // TODO(cohae): Order *might* be incorrect
    pub stage_pixel: SScopeStage,
    pub stage_vertex: SScopeStage,
    pub stage_geometry: SScopeStage,
    pub stage_hull: SScopeStage,
    pub stage_compute: SScopeStage,
    pub stage_domain: SScopeStage,
}

impl SScope {
    pub fn iter_stages(&self) -> impl Iterator<Item = &SScopeStage> {
        vec![
            &self.stage_pixel,
            &self.stage_vertex,
            &self.stage_geometry,
            &self.stage_hull,
            &self.stage_compute,
            &self.stage_domain,
        ]
        .into_iter()
    }
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0xffffffff)]
pub struct SScopeStage {
    pub unk0: [u32; 4],
    pub unk10: u64,
    pub bytecode: Vec<u8>,
    pub bytecode_constants: Vec<Vec4>,
    pub samplers: Vec<WideHash>,
    pub unk38: Vec<Vec4>,
    pub unk48: [u32; 4],

    pub constant_buffer_slot: i32,
    pub constant_buffer: TagHash,

    pub unk60: [u32; 6],
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x8080822D)]
pub struct SUnk8080822d {
    pub file_size: u64,
    pub unk8: Vec<u16>,
    pub unk18: Vec<Vec4>,
    pub unk28: Vec<()>,
}
