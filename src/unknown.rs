use crate::structure::TablePointer;
use crate::types::DestinyHash;
use binrw::BinRead;
use destiny_pkg::TagHash;

#[derive(BinRead, Debug, Clone)]
pub struct Unk80804f72 {
    pub file_size: u64,
    pub unk8: TablePointer<Unk80804f74>,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80804f74 {
    pub unk0: DestinyHash,
    pub unk4: DestinyHash,
    pub unk8: u64,
    pub unk10: TablePointer<Unk80804f76>,
    pub unk20: u64,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80804f76 {
    pub unk0: (TagHash, DestinyHash),
    pub unk8: (TagHash, DestinyHash),
    pub unk10: (TagHash, DestinyHash),
    pub unk18: (TagHash, DestinyHash),
}
