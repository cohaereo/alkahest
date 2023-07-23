use crate::structure::{RelPointer, TablePointer};
use crate::types::DestinyHash;
use binrw::BinRead;
use destiny_pkg::TagHash;

#[derive(BinRead, Debug)]
pub struct StringSetHeader {
    pub file_size: u64,
    pub string_hashes: TablePointer<DestinyHash>,
    pub language_english: TagHash,
    pub language_unk1: TagHash,
    pub language_german: TagHash,
    pub language_french: TagHash,
    pub language_unk4: TagHash,
    pub language_unk5: TagHash,
    pub language_italian: TagHash,
    pub language_unk7: TagHash,
    pub language_unk8: TagHash,
    pub language_unk9: TagHash,
    pub language_unk10: TagHash,
    pub language_polish: TagHash,
    pub language_unk12: TagHash,
}

#[derive(BinRead, Debug)]
pub struct StringData {
    pub file_size: u64,
    pub string_parts: TablePointer<StringPart>,
    pub _unk1: (u64, u64),
    pub _unk2: TablePointer<()>,
    pub string_data: TablePointer<u8>,
    pub string_combinations: TablePointer<StringCombination>,
}

#[derive(BinRead, Debug)]
pub struct StringCombination {
    pub data: RelPointer,
    pub part_count: i64,
}

#[derive(BinRead, Debug)]
pub struct StringPart {
    pub _unk0: u64,
    pub data: RelPointer,
    pub _unk1: u32,

    /// String data length.
    /// This is always equal to or larger than the string length due to variable character lengths
    pub byte_length: u16,
    pub string_length: u16,
    pub cipher_shift: u16,

    pub _unk2: u16,
    pub _unk3: u32,
}

/// Expects raw un-shifted data as input
/// Currently very incomplete
// TODO(cohae): Support for wide characters
pub fn decode_text(data: &[u8], cipher: u16) -> String {
    let mut result = String::new();

    let mut offset = 0;
    while offset < data.len() {
        let b0 = data[offset];
        let u0 = b0.wrapping_add(cipher as u8);

        match b0 {
            0..=0x7f => {
                result.push(char::from(u0));
                offset += 1
            }
            0xd0..=0xdf => {
                result.push(char::REPLACEMENT_CHARACTER);
                offset += 2
            }
            0xe0..=0xef => {
                result.push(char::REPLACEMENT_CHARACTER);
                offset += 3
            }
            0xf0..=0xfc => {
                result.push(char::REPLACEMENT_CHARACTER);
                offset += 4
            }
            _ => {
                result.push(char::REPLACEMENT_CHARACTER);
                offset += 1
            }
        }
    }

    result
}
