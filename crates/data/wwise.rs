use tiger_parse::tiger_type;
use tiger_pkg::TagHash;

#[tiger_type(id = 0x80809738)]
pub struct SWwiseEvent {
    pub file_size: u64,
    pub event_id: u32,
    pub unk_action_id: u32,

    pub unk10: u32,
    pub unk14: TagHash,
    pub wwise_bank: TagHash,
    pub unk1c: TagHash,
    pub wwise_streams: Vec<TagHash>,
}
