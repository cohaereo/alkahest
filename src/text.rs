use std::{
    io::{Cursor, Read, Seek, SeekFrom},
    ops::Deref,
    sync::Arc,
};

use alkahest_data::text::{SLocalizedStrings, SStringData, SStringPart};
use alkahest_pm::package_manager;
use destiny_pkg::TagHash;
use itertools::Itertools;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rustc_hash::FxHashMap;
use tiger_parse::{PackageManagerExt, TigerReadable};

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

pub type StringMapShared = Arc<GlobalStringmap>;
pub struct GlobalStringmap(pub FxHashMap<u32, String>);

impl GlobalStringmap {
    pub fn load() -> anyhow::Result<Self> {
        let _span = info_span!("Loading global strings").entered();
        let stringcontainers = package_manager()
            .get_all_by_reference(SLocalizedStrings::ID.unwrap())
            .into_iter()
            .filter(|(t, _)| {
                package_manager().package_paths[&t.pkg_id()]
                    .name
                    .contains("global")
            })
            .map(|(t, _)| t)
            .collect_vec();

        Ok(Self(
            stringcontainers
                .par_iter()
                .flat_map(|t| {
                    if let Ok(strings) = StringContainer::load(*t) {
                        strings.0.into_iter().collect_vec()
                    } else {
                        vec![]
                    }
                })
                .collect(),
        ))
    }

    pub fn get(&self, hash: impl Into<u32>) -> String {
        let hash = hash.into();
        self.0
            .get(&hash)
            .cloned()
            .unwrap_or_else(|| format!("[MISSING STRING: 0x{hash:08X}]"))
    }
}
