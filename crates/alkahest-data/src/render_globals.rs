use destiny_pkg::TagHash;
use glam::Vec4;
use tiger_parse::{tiger_tag, NullString, Pointer};

use super::Tag;
use crate::technique::SDynamicConstants;

#[tiger_tag(id = 0x80809780, size = 0x28)]
pub struct SClientBootstrap {
    #[tag(offset = 0x4c)]
    pub render_globals: Tag<SRenderGlobals>,
}

#[tiger_tag(id = 0x80806CB1)]
pub struct SRenderGlobals {
    pub file_size: u64,
    pub input_layouts: Tag<SVertexInputLayouts>,
    _padc: u32,
    pub scopes: Vec<SRenderGlobalScope>,
    pub pipelines: Vec<SRenderGlobalPipelines>,
    /// Lookup textures
    pub unk30: Tag<SUnk80806b99>,
    pub unk34: TagHash,
    pub unk38: TagHash,
}

#[derive(Debug)]
#[tiger_tag(id = 0x80806B99)]
pub struct SUnk80806b99 {
    pub file_size: u64,
    pub specular_tint_lookup_texture: TagHash,
    pub specular_lobe_lookup_texture: TagHash,
    pub specular_lobe_3d_lookup_texture: TagHash,
    pub iridescence_lookup_texture: TagHash,
}

#[derive(Debug)]
#[tiger_tag(id = 0x80806CB6)]
pub struct SRenderGlobalScope {
    pub name: Pointer<NullString>,
    pub unk8: u32,
    // TODO(cohae): Optional Tag<T>
    pub scope: TagHash,
}

#[derive(Debug)]
#[tiger_tag(id = 0x80806CB5)]
pub struct SRenderGlobalPipelines {
    pub name: Pointer<NullString>,
    pub unk8: u32,
    pub technique: TagHash,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x808071F3, size = 0x3b8)]

pub struct SScope {
    pub file_size: u64,
    pub name: Pointer<NullString>,

    #[tag(offset = 0x40)]
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
#[tiger_tag(id = 0xffffffff, size = 0x98)]
pub struct SScopeStage {
    pub unk0: [u32; 4],
    pub unk10: u64,
    pub constants: SDynamicConstants, // 0x18
    pub unk80: [u32; 6],
}

#[tiger_tag(id = 0x808072A6, size = 0x2c)]
pub struct SVertexInputLayouts {
    pub file_size: u64,
    pub unk8: u32,
    pub elements_c: Tag<SVertexInputElementSets>,
    pub elements_10: TagHash,
    pub elements_14: TagHash,
    pub elements_18: TagHash,
    pub elements_1c: TagHash,
    pub elements_20: TagHash,
    pub elements_24: TagHash,
    pub mapping: Tag<SVertexInputLayoutMapping>,
}

#[tiger_tag(id = 0x808072A9, size = 0x18)]
pub struct SVertexInputLayoutMapping {
    pub file_size: u64,
    pub layouts: Vec<SVertexLayout>,
}

#[tiger_tag(id = 0x808072AC, size = 0x1c)]
pub struct SVertexLayout {
    pub index: u8,

    #[tag(offset = 0x8)]
    pub element_0: u32,
    pub element_1: u32,
    pub element_2: u32,
    pub element_3: u32,

    pub unk18: u8,
    pub unk19: u8,
    pub unk1a: u8,
    pub unk1b: u8,
}

#[tiger_tag(id = 0x808072AD, size = 0x18)]
pub struct SVertexInputElementSets {
    pub file_size: u64,
    pub sets: Vec<SVertexInputElementSet>,
}

#[tiger_tag(id = 0x808072AF, size = 0x10)]
pub struct SVertexInputElementSet {
    pub elements: Vec<SVertexInputElement>,
}

#[tiger_tag(id = 0x808072B2, size = 3)]
pub struct SVertexInputElement {
    pub semantic: u8,
    pub semantic_index: u8,
    pub format: u8,
}
