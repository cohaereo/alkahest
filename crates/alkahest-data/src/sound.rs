use destiny_pkg::TagHash;
use tiger_parse::tiger_tag;

#[derive(Debug)]
#[tiger_tag(id = 0x80809738)]
pub struct SSoundCollection {
    pub file_size: u64,
    pub unk8: u32,
    pub unkc: u32,
    pub unk10: u32,
    pub unk14: TagHash,
    pub unk18: TagHash,
    pub unk1c: u32,
    pub streams: Vec<TagHash>,
    pub unk30: TagHash,
}
