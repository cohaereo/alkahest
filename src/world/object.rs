use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::LazyLock,
};

use ahash::AHashMap;
use alkahest_data::{hash::fnv1, tfx::features::dynamic::SDynamicModelComponent};
use alkahest_render::tfx::sequencer_vm::ObjectChannel;
use glam::Vec4;

pub struct PermutationConfig {
    pub permutation_index_override: Option<usize>,
    pub permutation_count: usize,

    /// Current configuration of key-value pairs
    pub configuration: HashMap<u32, u32>,

    /// Available values for each key
    keys: BTreeMap<u32, HashSet<u32>>,

    /// Maps from key-value pairs to permutation index
    pairs_to_permutation: HashMap<Vec<(u32, u32)>, usize>,
}

impl PermutationConfig {
    pub fn is_configurable(&self) -> bool {
        !self.keys.is_empty()
    }

    pub fn from_model(model: &SDynamicModelComponent) -> Option<Self> {
        let permutation_count = model
            .technique_map
            .iter()
            .filter(|m| m.unk8 == 0)
            .map(|m| m.technique_count as usize)
            .next()
            .unwrap_or(1);

        if model.unk408.is_empty() || model.unk418.is_empty() {
            // debug!(
            //     "TODO: Handle dynamic model permutations without unk408, dont know what to do \
            //      with these yet"
            // );
            return Some(Self {
                permutation_index_override: Some(0),
                permutation_count,
                configuration: Default::default(),
                keys: Default::default(),
                pairs_to_permutation: Default::default(),
            });
        }
        let mut configuration = HashMap::new();
        for keys1 in &model.unk38 {
            for keys in &keys1.unk8 {
                if keys.value == OPTION_KEY_INVALID {
                    continue;
                }
                configuration.insert(keys.switch_key, keys.value);
            }
        }

        let mut keys = BTreeMap::new();
        for u0 in &model.unk38 {
            for pair in &u0.unk8 {
                keys.entry(pair.switch_key)
                    .or_insert_with(HashSet::new)
                    .insert(pair.value);
            }
        }

        let mut pairs_to_permutation = HashMap::new();
        for (i, u) in model.unk418.iter().enumerate() {
            if u.unk2 < 0 {
                continue;
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

            let Some(key_indices) = model.unk408.get(start..end) else {
                error!("Invalid key indices range for permutation {}", i);
                return None;
            };
            for v in key_indices {
                let m = &model.unk38.get(*v as usize)?;
                keys.insert(m.unk8[0].switch_key, m.unk8[0].value);
            }

            let mut pair = keys.into_iter().collect::<Vec<(u32, u32)>>();
            pair.sort_by_key(|(k, _)| *k);
            pairs_to_permutation.insert(pair, i);
        }

        Some(Self {
            permutation_index_override: None,
            permutation_count,

            configuration,
            keys,
            pairs_to_permutation,
        })
    }

    pub fn iter_keys(&self) -> impl Iterator<Item = (u32, &HashSet<u32>)> {
        self.keys.iter().map(|(k, v)| (*k, v))
    }

    pub fn for_each_key_mut<F>(&mut self, mut f: F)
    where
        // key_hash, available_values, current_value
        F: FnMut(u32, &HashSet<u32>, &mut u32),
    {
        for (key, available_values) in &self.keys {
            if let Some(current_value) = self.configuration.get_mut(key) {
                f(*key, available_values, current_value);
            }
        }
    }

    pub fn calculate_permutation_index(&self) -> Option<usize> {
        if let Some(index) = self.permutation_index_override {
            return Some(index);
        }

        let mut key_vals: Vec<(u32, u32)> =
            self.configuration.iter().map(|(k, v)| (*k, *v)).collect();
        key_vals.sort_by_key(|(k, _)| *k);
        self.pairs_to_permutation.get(&key_vals).copied()
    }
}

pub struct ObjectChannels(pub Vec<ObjectChannel>);

impl ObjectChannels {
    pub fn set_by_name(&mut self, name: &str, value: Vec4) {
        let hash = fnv1(name);
        self.set_by_id(hash, value);
    }

    pub fn set_by_id(&mut self, hash: u32, value: Vec4) {
        if let Some(channel) = self.0.iter_mut().find(|c| c.name == hash) {
            channel.value = value;
        }
    }

    pub fn reset_usage_counters(&mut self) {
        for channel in &mut self.0 {
            channel.usage.store(0, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

const WORDLIST: &str = include_str!("wordlist.txt");

static FNV_NAMES: LazyLock<AHashMap<u32, String>> = LazyLock::new(|| {
    let mut map = AHashMap::new();
    for name in WORDLIST.lines() {
        if !name.is_empty() {
            map.insert(fnv1(name), name.to_string());
        }
    }
    map
});

pub const OPTION_KEY_INVALID: u32 = 0x871AC0EA;

const FNV_NAME_GUESSES: &[(u32, &str)] = &[
    (OPTION_KEY_INVALID, "<invalid>"),
    (0x20809827, "dark gray*"),
    (0xCFA916D2, "light gray*"),
    (0x9D102655, "white*"),
    (0x78532C1A, "olive*"),
    (0xDFF5552A, "dark green*"),
    (0x1023B2D3, "color*"),
];

pub fn find_fnv_name(hash: u32) -> Option<&'static str> {
    if let Some(s) = FNV_NAMES.get(&hash) {
        Some(s)
    } else {
        FNV_NAME_GUESSES
            .iter()
            .find(|&&(h, _)| h == hash)
            .map(|&(_, name)| name)
    }
}

pub fn find_fnv_name_or_default(hash: u32) -> String {
    find_fnv_name(hash).map_or_else(|| format!("unknown_{hash:08X}"), |v| v.to_string())
}
