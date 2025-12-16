use tiger_parse::{tiger_type, Padding};
use tiger_pkg::TagHash;

#[tiger_type(id = 0x80806D19)]
#[derive(Clone, Debug)]
pub struct SStaticAmbientOcclusion {
    pub file_size: u64,
    pub ao0: SAmbientOcclusionBuffer,
    pub ao1: SAmbientOcclusionBuffer,
    pub ao2: SAmbientOcclusionBuffer,
    // pub unk50: Vec<S80806D4F>,
    // pub unk60: Vec<u16>,
}

impl SStaticAmbientOcclusion {
    pub fn get_offset_by_identifier(&self, identifier: u64) -> Option<u32> {
        self.ao0
            .get_offset_by_identifier(identifier)
            .or_else(|| self.ao1.get_offset_by_identifier(identifier))
            .or_else(|| self.ao2.get_offset_by_identifier(identifier))
    }
}

#[tiger_type]
#[derive(Clone, Debug)]
pub struct SAmbientOcclusionBuffer {
    /// Vertex buffer
    pub buffer: TagHash,
    _padding: Padding<4>,
    pub mappings: Vec<SAmbientOcclusionOffsetMapping>,
}

impl SAmbientOcclusionBuffer {
    pub fn get_offset_by_identifier(&self, identifier: u64) -> Option<u32> {
        self.mappings
            .iter()
            .find(|mapping| mapping.identifier == identifier)
            .map(|mapping| mapping.offset)
    }
}

#[tiger_type(id = 0x80806D21)]
#[derive(Clone, Debug)]
pub struct SAmbientOcclusionOffsetMapping {
    pub identifier: u64,
    pub offset: u32,
    _pad: Padding<4>,
    pub unk10: [u32; 4],
}
