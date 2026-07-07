use std::{
    io::{Cursor, Read, Seek, SeekFrom},
    ops::Deref,
    sync::Arc,
};

use ahash::HashMap;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tiger_parse::{tiger_type, FnvHash, PackageManagerExt, Pointer, TigerReadable};
use tiger_pkg::{package_manager, TagHash};

#[derive(Debug)]
#[tiger_type(id = 0x808099EF)]
pub struct SLocalizedStrings {
    pub file_size: u64,
    pub string_hashes: Vec<FnvHash>,
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
#[tiger_type(id = 0xffffffff)]
pub struct SStringData {
    pub file_size: u64,
    pub string_parts: Vec<SStringPart>,
    // pub _unk1: (u64, u64),
    pub _unk2: Vec<()>,
    pub string_data: Vec<u8>,
    pub string_combinations: Vec<SStringCombination>,
}

#[derive(Debug)]
#[tiger_type(id = 0x808099F5)]
pub struct SStringCombination {
    pub data: Pointer<()>,
    pub part_count: i64,
}

#[derive(Debug)]
#[tiger_type(id = 0x808099F7)]
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
pub struct StringContainer(pub HashMap<u32, String>);

impl StringContainer {
    pub fn load(tag: impl Into<TagHash>) -> anyhow::Result<Self> {
        let mut stringmap = HashMap::default();
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
                cur.seek(SeekFrom::Start(combination.data.offset() as u64))?;
                cur.seek(SeekFrom::Current(ip * 0x20))?;
                let part: SStringPart = TigerReadable::read_ds(&mut cur)?;
                cur.seek(SeekFrom::Start(part.data.offset() as u64))?;
                let mut data = vec![0u8; part.byte_length as usize];
                cur.read_exact(&mut data)?;
                final_string += &String::from_utf8_lossy(&data);
            }

            stringmap.insert(*hash, final_string);
        }

        Ok(Self(stringmap))
    }

    pub fn load_all_global() -> Self {
        let stringcontainers: Vec<TagHash> = package_manager()
            .get_all_by_reference(SLocalizedStrings::ID.unwrap())
            .into_iter()
            .filter(|(t, _)| {
                package_manager()
                    .package_paths
                    .get(&t.pkg_id())
                    .is_some_and(|p| p.name.contains("global"))
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
            .unwrap_or_else(|| format!("<HASH(0x{hash:08X})>"))
    }

    pub fn merge(mut self, other: Self) -> Self {
        self.0.extend(other.0);
        self
    }
}

impl Deref for StringContainer {
    type Target = HashMap<u32, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
