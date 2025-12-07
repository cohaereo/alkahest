use glam::Vec4;
use tiger_parse::{tiger_type, NullString, Pointer};
use tiger_pkg::TagHash;

use crate::tag::Tag;

#[tiger_type(id = 0x8080978C)]
pub struct SRenderGlobals {
    pub file_size: u64,
    pub unk8: Vec<SUnk8080870f>,
    pub unk18: Vec<()>,
}

#[tiger_type(id = 0x8080870F)]
pub struct SUnk8080870f {
    pub unk0: u32,
    pub unk4: u32,
    pub unk8: Tag<SRenderGlobalsData>,
    pub unkc: u32,
}

#[tiger_type(id = 0x808067a8)]
pub struct SRenderGlobalsData {
    pub file_size: u64,
    pub input_layouts: Tag<SVertexInputLayouts>,
    _padc: u32,
    pub scopes: Vec<SRenderGlobalScope>,
    pub pipelines: Vec<SRenderGlobalPipelines>,
    /// Lookup textures
    pub unk30: Tag<SRenderGlobalLookupTextures>,
    pub global_channels: Tag<SRenderGlobalsGlobalChannels>,
    pub unk38: TagHash,
}

#[derive(Debug)]
#[tiger_type(id = 0x808066ae)]
pub struct SRenderGlobalLookupTextures {
    pub file_size: u64,
    pub specular_tint_lookup_texture: TagHash,
    pub specular_lobe_lookup_texture: TagHash,
    pub specular_lobe_3d_lookup_texture: TagHash,
    pub iridescence_lookup_texture: TagHash,
}

#[derive(Debug)]
#[tiger_type(id = 0x808067AD)]
pub struct SRenderGlobalScope {
    pub name: Pointer<NullString>,
    pub unk8: u32,
    // TODO(cohae): Optional Tag<T>
    pub scope: TagHash,
}

#[derive(Debug)]
#[tiger_type(id = 0x808067AC)]
pub struct SRenderGlobalPipelines {
    pub name: Pointer<NullString>,
    pub unk8: u32,
    pub technique: TagHash,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x8080822D)]
// cohae: I love this name
pub struct SRenderGlobalsGlobalChannels {
    pub file_size: u64,
    pub channel_ids: Vec<u32>,
    pub default_values: Vec<Vec4>,
    pub unk28: Vec<()>,
}

#[tiger_type(id = 0x80806D78, size = 0x38)]
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
    pub elements_28: TagHash,
    pub elements_2c: TagHash,
    pub mapping: Tag<SVertexInputLayoutMapping>,
}

#[tiger_type(id = 0x80806D7B, size = 0x18)]
pub struct SVertexInputLayoutMapping {
    pub file_size: u64,
    pub layouts: Vec<SVertexLayout>,
}

#[derive(Debug)]
#[tiger_type(id = 0x80806d7e, size = 0x1c)]
pub struct SVertexLayout {
    pub index: u8,

    #[tiger(offset = 0x8)]
    pub buffer_0: u32,
    pub buffer_1: u32,
    pub buffer_2: u32,
    pub buffer_3: u32,

    pub buffer_0_instanced: bool,
    pub buffer_1_instanced: bool,
    pub buffer_2_instanced: bool,
    pub buffer_3_instanced: bool,
}

#[tiger_type(id = 0x80806D7F, size = 0x18)]
pub struct SVertexInputElementSets {
    pub file_size: u64,
    pub sets: Vec<SVertexInputElementSet>,
}

#[tiger_type(id = 0x80806D81, size = 0x10)]
pub struct SVertexInputElementSet {
    pub elements: Vec<SVertexInputElement>,
}

#[tiger_type(id = 0x80806D84, size = 3)]
pub struct SVertexInputElement {
    pub semantic: u8,
    pub semantic_index: u8,
    pub format: u8,
}
