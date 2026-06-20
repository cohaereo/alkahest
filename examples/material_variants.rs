use std::{
    collections::{HashMap, HashSet},
    hash::{DefaultHasher, Hash, Hasher},
    io::{Cursor, Seek, SeekFrom},
    str::FromStr,
};

use alkahest_data::{
    hash::fnv1, pattern::SComponent, tfx::features::dynamic::SDynamicModelComponent,
};
use anyhow::Context;
use chroma_dbg::ChromaDebug;
use tiger_parse::{PackageManagerExt, TigerReadable};
use tiger_pkg::{TagHash, package_manager};

fn main() -> anyhow::Result<()> {
    let Some(hash) = std::env::args().nth(1) else {
        anyhow::bail!("Usage: technique_decompile <package dir> <technique tag>");
    };

    let Ok(hash) = TagHash::from_str(&hash) else {
        anyhow::bail!("Invalid technique tag hash: {}", hash);
    };

    alkahest_core::initialize_package_manager(None)?;

    let component: SComponent = package_manager()
        .read_tag_struct(hash)
        .context("Failed to read/parse tag")?;

    if component.default_instance.resource_type != 0x80808673 {
        panic!("Not a dynamic model component");
    }

    let mut cur = Cursor::new(package_manager().read_tag(hash)?);
    cur.seek(SeekFrom::Start(component.definition.offset))?;
    let model: SDynamicModelComponent = TigerReadable::read_ds(&mut cur)?;
    println!("{}", model.dbg_chroma());

    let mut defaults: HashMap<u32, u32> = HashMap::default();
    let mut variants_per_key: HashMap<u32, usize> = HashMap::new();
    for keys1 in &model.unk38 {
        for keys in &keys1.unk8 {
            println!(
                "Material Variant: {}={}",
                find_hash(keys.switch_key),
                find_hash(keys.value)
            );
            let entry = variants_per_key.entry(keys.switch_key).or_default();
            *entry += 1;
            defaults.entry(keys.switch_key).or_insert(keys.value);
        }
    }
    let mut total_variants = 1;
    for (key, count) in &variants_per_key {
        println!("Key {key:08X} has {count} variants");
        total_variants *= *count;
    }
    println!("Total possible variants: {}", total_variants);

    for (i, v) in model.unk408.iter().enumerate() {
        let m = &model.unk38[*v as usize];
        println!(
            "[{i:02}] -> {}={}",
            find_hash(m.unk8[0].switch_key),
            find_hash(m.unk8[0].value)
        );
    }

    let mut unique_states = HashSet::new();
    for (i, u) in model.unk418.iter().enumerate() {
        // let mut techniques = vec![];
        // for v in &model.technique_map {
        //     techniques.push(model.techniques[v.technique_start as usize + i]);
        // }

        println!(
            "Permutation {i:2}:",
            // techniques.iter().map(|v| v.to_string()).join(", ")
        );
        if u.unk2 < 0 {
            break;
        }
        // let start = u.unk6 as usize;
        // let end = start + (u.unk4 as usize);

        // let key_indices = &model.unk408[start..end];
        let mut keys: HashMap<u32, u32> = HashMap::default();
        // for v in key_indices {
        //     let m = &model.unk38[*v as usize];
        //     keys.insert(m.unk8[0].switch_key, m.unk8[0].value);
        // }

        let start = u.unk2 as usize;
        let end = start + (u.unk0 as usize);

        let key_indices = &model.unk408[start..end];
        for v in key_indices {
            let m = &model.unk38[*v as usize];
            keys.insert(m.unk8[0].switch_key, m.unk8[0].value);
        }

        for (k, val) in &keys {
            println!("    {}={}", find_hash(*k), find_hash(*val));
        }
        let hash = {
            let mut hasher = DefaultHasher::new();
            let mut key_vals: Vec<(u32, u32)> = keys.into_iter().collect();
            key_vals.sort_by_key(|(k, _)| *k);
            for (k, v) in key_vals {
                k.hash(&mut hasher);
                v.hash(&mut hasher);
            }
            hasher.finish()
        };
        unique_states.insert(hash);
        println!("    hash=0x{hash:08X}");
        println!();
    }

    println!("Unique material states: {}", unique_states.len());

    Ok(())
}

const HASH_STRS: &[&str] = &[
    "accent",
    "black",
    "blue",
    "brightness",
    "cyan",
    "device_lock",
    "device_position",
    "device_power",
    "dim",
    "flicker",
    "flux",
    "gray",
    "grey",
    "grayscale",
    "green",
    "grime",
    "high",
    "imminencies",
    "interference",
    "inverted",
    "main",
    "max",
    "mid",
    "min",
    "off",
    "off",
    "on",
    "orange",
    "permutation",
    "pink",
    "pulse",
    "red",
    "region",
    "smoothness",
    "struggle",
    "uv",
    "waterline",
    "white",
    "yellow",
    "frost",
];

const GUESSES: &[(u32, &str)] = &[(0xB01748DA, "color"), (0x871AC0EA, "<unused>")];

fn find_hash(hash: u32) -> String {
    if let Some(s) = HASH_STRS.iter().find(|s| fnv1(s) == hash) {
        s.to_string()
    } else if let Some((_, s)) = GUESSES.iter().find(|(h, _)| *h == hash) {
        s.to_string()
    } else {
        format!("{:08X}", hash)
    }
}
