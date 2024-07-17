use std::{
    io::{Cursor, Read, Seek, SeekFrom},
    ops::Deref,
    sync::Arc,
};

use alkahest_pm::package_manager;
use destiny_pkg::TagHash;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rustc_hash::FxHashMap;
use tiger_parse::{tiger_tag, PackageManagerExt, Pointer, TigerReadable};

use crate::common::ResourceHash;

#[derive(Debug)]
#[tiger_tag(id = 0x808099EF)]
pub struct SLocalizedStrings {
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
pub struct SStringData {
    pub file_size: u64,
    pub string_parts: Vec<SStringPart>,
    // pub _unk1: (u64, u64),
    pub _unk2: Vec<()>,
    pub string_data: Vec<u8>,
    pub string_combinations: Vec<SStringCombination>,
}

#[derive(Debug)]
#[tiger_tag(id = 0x808099F5)]
pub struct SStringCombination {
    pub data: Pointer<()>,
    pub part_count: i64,
}

#[derive(Debug)]
#[tiger_tag(id = 0x808099F7)]
pub struct SStringPart {
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

pub type StringContainerShared = Arc<StringContainer>;
#[derive(Default)]
pub struct StringContainer(pub FxHashMap<u32, String>);

impl StringContainer {
    pub fn load(tag: impl Into<TagHash>) -> anyhow::Result<Self> {
        let mut stringmap = FxHashMap::default();
        let textset_header: SLocalizedStrings = package_manager().read_tag_struct(tag)?;

        let data = package_manager()
            .read_tag(textset_header.language_english)
            .unwrap();
        let mut cur = Cursor::new(&data);
        let text_data: SStringData = TigerReadable::read_ds(&mut cur)?;

        for (combination, hash) in text_data
            .string_combinations
            .iter()
            .zip(textset_header.string_hashes.iter())
        {
            let mut final_string = String::new();

            for ip in 0..combination.part_count {
                cur.seek(SeekFrom::Start(combination.data.offset()))?;
                cur.seek(SeekFrom::Current(ip * 0x20))?;
                let part: SStringPart = TigerReadable::read_ds(&mut cur)?;
                cur.seek(SeekFrom::Start(part.data.offset()))?;
                let mut data = vec![0u8; part.byte_length as usize];
                cur.read_exact(&mut data)?;
                final_string += &decode_text(&data, part.cipher_shift);
            }

            stringmap.insert(hash.0, final_string);
        }

        Ok(Self(stringmap))
    }

    pub fn load_all_global() -> Self {
        let stringcontainers: Vec<TagHash> = package_manager()
            .get_all_by_reference(SLocalizedStrings::ID.unwrap())
            .into_iter()
            .filter(|(t, _)| {
                package_manager().package_paths[&t.pkg_id()]
                    .name
                    .contains("global")
            })
            .map(|(t, _)| t)
            .collect();

        Self(
            stringcontainers
                .par_iter()
                .flat_map(|t| {
                    if let Ok(strings) = StringContainer::load(*t) {
                        strings.0.into_iter().collect()
                    } else {
                        vec![]
                    }
                })
                .collect(),
        )
    }

    pub fn try_get(&self, hash: impl Into<u32>) -> Option<String> {
        let hash = hash.into();
        self.0.get(&hash).cloned()
    }

    pub fn get(&self, hash: impl Into<u32>) -> String {
        let hash = hash.into();
        self.try_get(hash)
            .unwrap_or_else(|| format!("[MISSING STRING: 0x{hash:08X}]"))
    }

    pub fn merge(mut self, other: Self) -> Self {
        self.0.extend(other.0);
        self
    }
}

impl Deref for StringContainer {
    type Target = FxHashMap<u32, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Expects raw un-shifted data as input
/// Currently very incomplete
// TODO(cohae): Support for wide characters
pub fn decode_text(data: &[u8], cipher: u16) -> String {
    if cipher == 0 {
        return String::from_utf8_lossy(data).to_string();
    }

    let mut result = String::new();

    let mut offset = 0;
    while offset < data.len() {
        let b0 = data[offset];
        let u0 = b0.wrapping_add(cipher as u8);

        match b0 {
            0xc0..=0xdf => {
                result.push(char::REPLACEMENT_CHARACTER);
                offset += 2
            }
            0xe0..=0xef => {
                result.push(char::REPLACEMENT_CHARACTER);
                offset += 3
            }
            0..=0x7f => {
                result.push(char::from(u0));
                offset += 1
            }
            _ => {
                result.push(char::REPLACEMENT_CHARACTER);
                offset += 1
            }
        }
    }

    result
}
