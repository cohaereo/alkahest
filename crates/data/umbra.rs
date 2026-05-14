use glam::Vec4;
use tiger_parse::tiger_type;
use tiger_pkg::TagHash;

#[tiger_type(id = 0x80806E6B, size = 0xE8)]
pub struct SUmbraTomes {
    pub file_size: u64,
    pub tome0: TagHash,
    pub tome1: TagHash,
    pub object_bindings: TagHash,

    #[tiger(offset = 0x20)]
    pub unk20: Vec<()>,
    pub unk30: Vec<()>,

    #[tiger(offset = 0x60)]
    pub unk60: Vec4,
    pub unk70: Vec4,
    pub unk80: Vec4,
    pub unk90: Vec<()>,
    pub unka0: TagHash,
    pub unka4: TagHash,
    pub unka8: TagHash,

    #[tiger(offset = 0xB0)]
    pub unkb0: Vec<()>,

    #[tiger(offset = 0xE0)]
    pub unke0: TagHash,
    pub unke4: TagHash,
}
