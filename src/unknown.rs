use crate::{structure::TablePointer, types::ResourceHash};
use binrw::BinRead;
use destiny_pkg::TagHash;

#[derive(BinRead, Debug, Clone)]
pub struct Unk80804f72 {
    pub file_size: u64,
    pub unk8: TablePointer<Unk80804f74>,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80804f74 {
    pub unk0: ResourceHash,
    pub unk4: ResourceHash,
    pub unk8: u64,
    pub unk10: TablePointer<Unk80804f76>,
    pub unk20: u64,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80804f76 {
    pub unk0: (TagHash, ResourceHash),
    pub unk8: (TagHash, ResourceHash),
    pub unk10: (TagHash, ResourceHash),
    pub unk18: (TagHash, ResourceHash),
}
