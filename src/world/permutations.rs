use std::collections::{BTreeMap, HashMap, HashSet};

use alkahest_data::{hash::fnv1, tfx::features::dynamic::SDynamicModelComponent};

pub struct PermutationConfig {
    /// Current configuration of key-value pairs
    pub configuration: HashMap<u32, u32>,

    /// Available values for each key
    keys: BTreeMap<u32, HashSet<u32>>,

    /// Maps from key-value pairs to permutation index
    pairs_to_permutation: HashMap<Vec<(u32, u32)>, usize>,
}

impl PermutationConfig {
    pub fn from_model(model: &SDynamicModelComponent) -> Option<Self> {
        if model.unk408.is_empty() || model.unk418.is_empty() {
            debug!(
                "TODO: Handle dynamic model permutations without unk408, dont know what to do \
                 with these yet"
            );
            return None;
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
        let mut key_vals: Vec<(u32, u32)> =
            self.configuration.iter().map(|(k, v)| (*k, *v)).collect();
        key_vals.sort_by_key(|(k, _)| *k);
        self.pairs_to_permutation.get(&key_vals).copied()
    }
}

#[rustfmt::skip]
const FNV_NAMES: &[&str] = &[
    "-1", ".25", ".5", ".75", "0", "0.001", "0.25", "0.3", "0.4", "0.5", "0.7", "01", "02", "03", "04", "05", "06", "1", "1.5", "10", "100", "10m", "10x10", "10x30", "15m", "1x1", "2", "200", "20m", "25", "2x", "2x5m", "3", "3x", "4", "40m", "4m", "5", "50", "500", "5m", "5x10m", "5x5m", "75", "75m", "90", "a", "accent_orange", "acolyte", "active", "alert", "alive", "all", "alpha_bravo", "amber", "anim", "animated", "animation", "antenna_left", "ao", "ao_value", "aphix_invasive", "apple", "aqua", "arc", "arm", "arm_left", "arm_right", "armed", "armor", "armor_on", "arms", "arms_left", "arms_right", "ash", "aspect_ratio", "asphalt", "atmosphere", "attendant", "attendant_base", "attract", "b", "babies", "back", "back_garbage", "badge", "bael", "ball", "banana", "bank", "barnacles", "barriers", "base", "basilisk", "basilisk_major", "bat", "battle_armor", "beach_ball", "beacon", "beat", "behemoth", "belt", "bender", "bg2", "billow", "bishop", "black", "blend", "blocked", "blood_of_oryx", "blue", "body", "body_right", "bomb_state", "bone_crushers", "boss", "boss_geo", "both", "bottom", "bounty", "bowl", "box", "brand", "brass", "breathe", "brick", "bridge", "bright", "brighter", "brightness", "broken", "bronze", "brown", "bruiser", "bruiser_base", "bruiser_loyalist", "bubble", "bulbs", "bun", "bunker", "burn", "button", "c", "cabal", "cabal_ship", "caiatl", "captain", "captain_base", "card", "carved", "cayde", "censer", "centurion", "centurion_base", "centurion_bfg", "centurion_loyalist", "chaingun", "chains", "change", "charger", "cheap", "cheer", "chest", "chest_garbage", "chin", "chocolate", "chrome", "chunks", "civilian_female01", "civilian_female02", "civilian_female03", "civilian_maint_male01", "civilian_maint_male02", "civilian_male01", "civilian_male02", "civilian_male03", "civilian_mil_fem01", "civilian_mil_male01", "clean", "cliffs", "cloak_in", "closed", "cloth", "coin", "collar", "collector", "color", "coloring", "combat", "complete", "concrete", "condition", "cone", "convert", "cool", "cool_white", "copper", "core", "cosmic", "cosmo", "cover", "cracked", "crate", "crimson", "crossbones", "crota", "crow", "crown", "crucible", "crusader", "cryo", "cryptarch", "crystal", "crystal_state", "cube", "cubemap", "cuboid", "curio", "curse", "cyan", "cycle", "cyclops", "cyclops_base", "cyclops_sol_divisive", "cylinder", "d", "daito", "damage", "damage_light", "damage_type", "damageable", "damaged", "dark", "dark blue", "dark grey", "dark red", "dark_cabal", "dark_gray", "dark_grey", "dark_teal", "dark_violet", "darker", "darkest", "darkgray", "darkness", "dead", "dead_king", "dead_orbit", "debug_set_active_detain", "decal", "decals", "deep_teal", "deep_violet", "default", "defender", "deja", "delete", "destination", "destroyable", "destroyed", "destructable", "destructible", "detain_active", "device", "devils", "diamond", "dim", "dirt", "disable", "disabled", "dismay", "display", "distort", "door", "door_closed", "double", "down", "dps_active", "dps_exit", "dps_immune", "dreg", "dreg_base", "droop", "dusk", "dust_giants", "dye", "e", "ear", "echo", "eden", "edz", "eggs", "eight", "elite", "emblem", "emblems", "emissive", "emitter_shape", "emotes", "empty", "enable", "enabled", "epiphany", "eramis", "erosion", "europa", "event", "eververse", "exile", "exotic_mission", "exterior", "eye", "f", "fabric", "face", "facing", "faction", "fade", "failsafe", "faint", "falcon", "fallen", "false", "fanatic", "fanatic_base", "fanatic_future", "fanatic_past", "far", "fast", "female", "fin", "finish", "finisher_dummy", "fire_on", "five", "flag_color", "flat", "flayer", "flicker", "flip", "floor", "flower", "flux", "fog_density", "forsaken", "fotc", "four", "frame", "front", "front_plate", "front_shield", "frosted", "full", "future", "fwc", "fx", "g", "garbage", "garden", "gas_tank", "gate_lord", "gear_loaded", "gender", "generic", "ghost", "ghost_1", "ghost_2", "ghost_3", "ghost_4", "gladiator", "gladiator_base", "gladiator_loyalist", "glancing", "glass", "glimmer", "glossy", "glow", "goblin", "goblin_base", "goblin_future", "goblin_past", "gold", "goliath", "gondola", "gradient", "graphic", "grass", "grate", "gravel", "gray", "green", "grey", "grey_blue", "grid", "ground", "grow", "grunge", "gun", "h", "hair", "hakke", "half", "hand", "harpy", "harpy_base", "harpy_future", "harpy_past", "harvester", "haul", "head", "heartbeat", "heat", "heavy", "heavy_shank", "height", "helmet", "hemisphere", "hezen_corrective", "hezen_prime", "hidden_swarm", "high", "highlight", "hive", "hobgoblin", "hobgoblin_base", "hobgoblin_future", "hobgoblin_past", "holidays", "holo", "holster", "hood", "horn", "house_of_devils", "house_of_dusk", "house_of_exile", "house_of_kings", "house_of_steel", "house_of_winter", "house_of_wolves", "hover", "hub", "hull", "hull_front", "hum", "hunter", "husk", "hymn_exp_ultra_taken_phalanx", "i", "ice", "ice_reapers", "illum", "illum_intensity", "immune", "immune_state", "inactive", "indicator_type", "infection", "inflate", "inside", "intact", "intensity", "interactable", "interior", "intersection_back", "intersection_front", "intro", "invader", "invert", "inverted", "invulnerable", "iridescent", "iron", "ivory", "j", "jade", "jetpack", "jiggle", "jitter", "judgment", "juice_box", "juicebox", "k", "killer", "kinetic", "king", "kings", "knife", "knight", "knight_base", "knight_cleaver", "knight_major", "knight_miniboss", "knight_shield", "knight_ultra", "l", "labyrinth", "lantern", "lantern_state", "large", "laser_blasters", "laser_rifle", "laurel_wreath", "left", "left_arm", "left_back", "left_leg", "left_shield", "left_shoulder", "left_shoulder_garbage", "leg_left", "leg_right", "legionary", "legionary_base", "legionary_loyalist", "legionary_shotgun", "legionary_sniper", "legs", "lesser_cry_for_vengeance", "lichen", "lift", "light", "light color", "light green", "light intensity", "light1", "light_behavior", "light_blue", "light_color", "light_gray", "light_intensity", "light_seq", "light_state", "light_trigger", "lightblue", "lighter", "lighting", "lime", "loam", "local", "location", "logo", "long", "low", "low_wind", "lower_left_wing", "lower_right_wing", "loyalist", "lucent", "luna", "lute_exp_ultra_minotaur", "m", "mace_grenade", "macro", "magenta", "main", "major", "major_blocker", "male", "mamba_role", "map1", "map2", "map3", "mapping", "marauder", "marauder_base", "mask", "material", "matte", "max", "maya", "mboss_minotaur", "med", "medium", "melee", "memorial", "metal", "metallic", "mid", "middle", "min", "miniboss", "minor", "minotaur", "minotaur_atheon", "minotaur_base", "minotaur_future", "minotaur_past", "minotaur_protheon", "mint", "mirror", "missile_launcher", "mission_voyage", "modifier", "monastery", "morph", "mortar", "moss", "mote_loss", "motors", "mud", "n", "neck", "necklace", "neolithic", "neutral", "new_monarchy", "nimbus", "nine", "no", "no_faction", "non", "none", "normal", "normal_frame", "nothing", "o", "occupied", "oculus", "off", "offset", "ogre", "ogre_base", "ogre_hidden_swarm", "oil", "omnigul", "omolon", "on", "one", "onyx", "onyx_color", "ooze", "opacity", "opaque", "open", "orange", "orientation", "osiris", "outer_shell", "overlay", "overload", "overworld", "pack", "packed", "painted", "panel", "panels", "parallax", "part", "parts", "past", "path", "pattern", "pawn", "payload", "permutation", "petra_knife", "phalanx", "phalanx_base", "phalanx_loyalist", "phase_in", "physics", "physics_on", "piccolo_marauder", "piece", "pigeon", "pillar", "pink", "pixel", "plaster", "playable", "player", "plinth", "plywood", "point", "polaris_ice", "polaris_rock", "pose", "position", "position_off", "position_on", "postmaster", "powder", "powder_blue", "present", "primus_shield", "pristine", "progress_bar", "projectiles_only", "proximity", "psion", "psion_base", "psion_boss", "psion_council_red", "psion_general_arc", "psion_loyalist", "psion_major", "psion_miniboss", "pulse", "purple", "pyramid", "pyramid_dark", "pyramid_geo", "pyro", "pyro_base", "pyro_loyalist", "quarter", "queen", "quest", "quests", "quicksand", "quiver", "race", "rack", "raid_vendor", "rainbow", "ramen", "random", "ranged", "raspberry", "raycast", "rectangle", "red", "red_guard", "red_orange", "redguard", "redjack", "reflection", "region", "regret", "relic", "render", "revenant", "revenge_commander", "reverse", "rez_in", "right", "right_arm", "right_back", "right_leg", "right_shield", "right_shoulder", "right_shoulder_garbage", "ring", "rings", "robes", "rock", "rockets", "rook", "rotate", "rotation", "rubble", "rune", "rune_a", "rune_b", "rune_c", "rune_d", "rune_e", "rust", "rusty", "s24", "s25", "salt", "sand", "sand_eaters", "scale", "scimitar", "scimitar_on", "scourge", "screen", "second_left", "second_right", "section", "security", "seed", "servitor_base", "servitor_house_of_dusk", "seven", "sfx_reprojection_mipmap_hdr", "shaders", "shadow_thrall", "shadows", "shank", "shank_exploder", "shank_repeater", "shank_tracer", "shape", "shaxx", "shell", "shell_on", "shield", "shield_bottom_left", "shield_bottom_right", "shield_down", "shield_extended", "shield_off", "shield_on", "shield_retracted", "shield_taken", "shield_top_left", "shield_top_right", "shield_vex", "shielded", "shiny", "ships", "shire", "shoreline", "short", "shot", "shoulder_left", "shoulder_right", "shutters", "siege_dancers", "silent_brood", "silver", "sim", "six", "size", "sizes", "skulls", "sky", "sky color override", "sky_burners", "skyburners", "slag_base", "slope", "slow", "small", "small_01", "small_02", "snow", "soccer_ball", "soft_blue", "soft_orange", "sol_divisive", "solar", "soldier", "solid", "soot", "space", "spawn", "spawn_of_crota", "speaker", "spear", "special", "sphere", "spheroid", "spider", "spider_shank", "spike", "spin", "spire", "spirit", "splash", "spring", "sputter", "squiggle", "standard", "starfruit", "start", "stasis", "state", "static", "step", "stone", "strawberry", "streamer", "strike", "strike_bond", "style", "subclass", "subjugator_dragoon", "subjugator_dragoon_boss", "subjugator_mage", "subjugator_mage_boss", "sun transmission color", "super_interceptor", "suros", "swap", "swell", "swirl", "sword", "symbol", "symbol_a", "symbol_b", "symbol_c", "symbol_d", "syndicate", "t0", "tail", "taken", "taken_base", "taken_centurion_base", "taken_goblin_base", "taken_hobgoblin_base", "taken_knight_base", "taken_minotaur_base", "taken_ogre_base", "taken_phalanx_base", "taken_psion_base", "taken_thrall_base", "taken_trooper_base", "taken_vandal_base", "taken_wizard_base", "tall", "talus", "tan", "taniks", "tank_startup", "tape", "tar", "target", "teal", "tech_witch", "tech_witch_cleansed", "tech_witch_vendor", "techeun", "teleporter", "teleporter_active", "teleporter_on", "teleporter_state", "ten", "terraform", "the_iron_alliance", "the_smoke_bomb_sticks_to_surfaces_and", "thermal", "thin", "third_inner", "third_mid", "third_outer", "thrall", "thrall_base", "thrall_exploder", "thrall_exploder_hidden_swarm", "thrall_hidden_swarm", "three", "thrusters", "tight", "tile", "tiles", "tiling", "tint", "titan", "titan_base", "titan_future", "titan_past", "titan_present", "titan_sol_divisive", "titan_templar", "top", "top_head", "top_wing", "tower", "track", "transition", "transmat", "transmission", "treatment", "triad", "triad_bravo", "triad_charlie", "triad_delta", "triangle_prism", "tribute", "trinkets", "trooper", "trooper_base", "true", "tube", "turquoise", "turret", "turret_bottom_left", "turret_bottom_right", "turret_top_left", "turret_top_right", "twitch", "two", "type", "ultra", "ultra_minotaur_a", "ultra_ogre_a", "ultra_shank", "ultraviolet", "undamaged", "underworld", "undying_mind", "unit", "unit_type", "up", "upperarm", "v", "valkyrie_base", "valkyrie_future", "valkyrie_past", "valkyrie_quria", "vandal", "vandal_base", "vanguard", "vanilla", "variable", "variant", "vehicles", "vendor", "vendor_black_market", "vendor_crm", "vendor_cryptarch", "vendor_dead_orbit", "vendor_future_war_cult", "vendor_new_monarchy", "vendor_pvp", "vendor_shaders", "vendor_ships", "vendor_vanguard", "vendor_weapons", "venom_of_oryx", "vertex", "vertical", "very_far", "vex", "violet", "viper_centurion", "viper_centurion_loyalist", "viper_legionary", "viper_legionary_loyalist", "viper_pyro", "viper_pyro_loyalist", "virgo_prohibition", "vis", "visibility", "visibility_on", "visible", "visible_in", "void", "volume", "vulnerable", "wall", "wander", "warbeast", "warbeast_base", "warbeast_loyalist", "warlock", "warm", "warm_white", "warning", "water_damage", "waterline", "weakspot_on", "weapon_type", "weapons", "weaver", "weaver_base", "wet", "wetness", "white", "white-red", "white_blue", "white_frame", "whole", "wide", "width", "wiggle", "wind", "windshield", "wings", "winter", "wire", "witness", "wizard", "wizard_base", "wizard_hidden_swarm", "wolves", "worn", "x", "yellow", "yes", "zero", "zone",
];

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

fn find_fnv_name(hash: u32) -> Option<&'static str> {
    if let Some(s) = FNV_NAMES
        .iter()
        .find(|&&name| fnv1(name.as_bytes()) == hash)
        .copied()
        .map(|v| v as _)
    {
        Some(s)
    } else {
        FNV_NAME_GUESSES
            .iter()
            .find(|&&(h, _)| h == hash)
            .map(|&(_, name)| name)
    }
}

pub fn find_kv_name(hash: u32) -> Option<&'static str> {
    find_fnv_name(hash)
}

pub fn find_kv_name_or_default(hash: u32) -> String {
    find_kv_name(hash)
        .map(|v| v.to_string())
        .unwrap_or_else(|| format!("unknown_{hash:08X}"))
}
