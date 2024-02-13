use destiny_pkg::TagHash;
use tiger_parse::tiger_tag;

use crate::common::ResourceHash;

#[derive(Debug, Clone)]
#[tiger_tag(id = 0xffffffff, size = 0x30)]
pub struct Unk80804f72 {
    pub file_size: u64,
    pub unk8: Vec<Unk80804f74>,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0xffffffff, size = 0x30)]
pub struct Unk80804f74 {
    pub unk0: ResourceHash,
    pub unk4: ResourceHash,
    pub unk8: u64,
    pub unk10: Vec<Unk80804f76>,
    pub unk20: u64,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0xffffffff, size = 0x30)]
pub struct Unk80804f76 {
    pub unk0: (TagHash, ResourceHash),
    pub unk8: (TagHash, ResourceHash),
    pub unk10: (TagHash, ResourceHash),
    pub unk18: (TagHash, ResourceHash),
}
