use destiny_pkg::TagHash;
use tiger_parse::{tiger_tag, Pointer};

use crate::common::ResourceHash;

#[derive(Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct StringContainer {
    pub file_size: u64,
    pub string_hashes: Vec<ResourceHash>,
    pub language_english: TagHash,
    // pub language_unk1: TagHash,
    // pub language_german: TagHash,
    // pub language_french: TagHash,
    // pub language_unk4: TagHash,
    // pub language_unk5: TagHash,
    // pub language_italian: TagHash,
    // pub language_unk7: TagHash,
    // pub language_unk8: TagHash,
    // pub language_unk9: TagHash,
    // pub language_unk10: TagHash,
    // pub language_polish: TagHash,
    // pub language_unk12: TagHash,
}

#[derive(Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct StringData {
    pub file_size: u64,
    pub string_parts: Vec<StringPart>,
    // pub _unk1: (u64, u64),
    pub _unk2: Vec<()>,
    pub string_data: Vec<u8>,
    pub string_combinations: Vec<StringCombination>,
}

#[derive(Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct StringCombination {
    pub data: Pointer<()>,
    pub part_count: i64,
}

#[derive(Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct StringPart {
    pub _unk0: u64,
    pub data: Pointer<()>,
    pub _unk1: u32,

    /// String data length.
    /// This is always equal to or larger than the string length due to variable character lengths
    pub byte_length: u16,
    pub string_length: u16,
    pub cipher_shift: u16,

    pub _unk2: u16,
    pub _unk3: u32,
}
