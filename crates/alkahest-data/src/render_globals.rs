use destiny_pkg::TagHash;
use tiger_parse::{tiger_tag, NullString, Pointer};

use super::{Tag, WideHash};

#[derive(Debug)]
#[tiger_tag(id = 0x8080978C)]
pub struct SRenderGlobals {
    pub file_size: u64,
    pub unk8: Vec<Unk8080870f>,
    pub unk18: Vec<()>,
}

#[derive(Debug)]
#[tiger_tag(id = 0x8080870F)]
pub struct Unk8080870f {
    pub unk0: u32,
    pub unk4: u32,
    pub unk8: Tag<Unk808067a8>,
    pub unkc: u32,
}

#[derive(Debug)]
#[tiger_tag(id = 0x808067a8)]
pub struct Unk808067a8 {
    pub file_size: u64,
    pub unk8: TagHash,
    _pad10: u32,
    pub scopes: Vec<Unk808067ad>,
    pub unk20: Vec<Unk808067ac>,
    /// Lookup textures
    pub unk30: Tag<Unk808066ae>,
    pub unk34: TagHash,
    pub unk38: TagHash,
}

#[derive(Debug)]
#[tiger_tag(id = 0x808066ae)]
pub struct Unk808066ae {
    pub file_size: u64,
    pub specular_tint_lookup_texture: TagHash,
    pub specular_lobe_lookup_texture: TagHash,
    pub specular_lobe_3d_lookup_texture: TagHash,
    pub iridescence_lookup_texture: TagHash,
}

#[derive(Debug)]
#[tiger_tag(id = 0x808067AD)]
pub struct Unk808067ad {
    pub name: Pointer<NullString>,
    pub unk8: u32,
    pub scope: Tag<SScope>,
}

#[derive(Debug)]
#[tiger_tag(id = 0x808067AC)]
pub struct Unk808067ac {
    pub name: Pointer<NullString>,
    pub unk8: u32,
    pub technique: TagHash,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x80806DBA, size = 0x400)]

pub struct SScope {
    pub file_size: u64,
    pub name: Pointer<NullString>,

    #[tag(offset = 0x58)]
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
    pub unk0: u64,
    pub bytecode: Vec<u8>,
    pub bytecode_constants: Vec<glam::Vec4>,
    pub samplers: Vec<WideHash>,
    pub unk38: Vec<glam::Vec4>,
    pub unk48: [u32; 4],

    pub constant_buffer_slot: i32,
    pub constant_buffer: TagHash,

    pub unksomething: [u32; 10],
}
