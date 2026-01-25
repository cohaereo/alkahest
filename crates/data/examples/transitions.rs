use ahash::HashSet;
use alkahest_data::{activity::transition::S808096BF, hash::fnv1, strings::StringContainer};
use chroma_dbg::ChromaDebug;
use tiger_parse::PackageManagerExt;
use tiger_pkg::package_manager;

fn main() -> anyhow::Result<()> {
    alkahest_core::initialize_package_manager(None)?;

    let strings = StringContainer::load(0x80A687EC)?;
    let p: S808096BF = package_manager().read_tag_struct(0x80BB313E)?;
    // let strings = StringContainer::load(0x80D409BF)?;
    // let p: S808096BF = package_manager().read_tag_struct(0x8131FEF6)?;
    println!("{}", p.dbg_chroma());
    let mut transition_indices = HashSet::default();
    let mut orphaned_indices = HashSet::default();
    let mut orphaned_unk10 = HashSet::default();

    for (i, s) in p.unk10.iter().enumerate() {
        println!(
            "{i}: {} ({:?}) - 0x{:016X}",
            find_string_fnv(s.name),
            s.kind,
            s.unk0
        );
        orphaned_unk10.insert(i);
    }

    for (i, s) in p.bubbles.iter().enumerate() {
        // println!("{i}: {} {}", find_string(&strings, s.name), s.dbg_chroma());
        orphaned_indices.insert(i);
        orphaned_unk10.remove(&(s.index_into_unk10 as usize));
        // println!("  transitions:");
        for u in &s.transitions {
            transition_indices.insert(u.unk0 as usize);
            orphaned_unk10.remove(&(u.unk0 as usize));
            // println!(
            //     "    - {}",
            //     find_string(&strings, p.unk10[u.unk0 as usize].name)
            // );
        }
        // println!("  dependencies:");
        for u in &s.dependencies {
            orphaned_unk10.remove(&(u.unk0 as usize));
            //     transition_indices.insert(u.unk0 as usize);
            //     println!(
            //         "    - {}",
            //         find_string(&strings, p.unk10[u.unk0 as usize].name)
            //     );
        }
    }

    // let mut id_to_name = HashMap::default();
    // for (i, v) in p.unk10.iter().enumerate() {
    //     id_to_name.insert(v.unk0, find_string_fnv(v.name));
    //     println!(
    //         "{i}: {}, {:016X}",
    //         find_string(&strings, v.name),
    //         v.unk0.to_be()
    //     );
    // }

    // let p2: S80809567 = package_manager().read_tag_struct(0x80B970AE)?;
    // // println!("{}", p2.dbg_chroma());

    // for (trigger, volume) in p2.unk28.iter().zip(p2.unk90.entries.iter()) {
    //     println!(
    //         "0x{:X} ({:?}) -> 0x{:X} ({:?})",
    //         trigger.unk8,
    //         id_to_name.get(&trigger.unk8),
    //         trigger.unk18,
    //         id_to_name.get(&trigger.unk18)
    //     );
    //     println!("  v0 {:?}", volume.v0);
    //     println!("  v1 {:?}", volume.v1);
    //     println!("  v2 {:?}", volume.v2);
    //     for s in &volume.warnings {
    //         println!("  build warning: {}", s.to_string());
    //     }
    //     println!();
    // }

    // for (tag, _) in package_manager().get_all_by_reference(0x80809567) {
    //     let Ok(p2) = package_manager().read_tag_struct::<S80809567>(tag) else {
    //         continue;
    //     };
    //     for e in &p2.unk90.entries {
    //         for s in &e.warnings {
    //             println!(
    //                 "{tag} ({}):  build warning: {}",
    //                 package_manager().package_paths[&tag.pkg_id()].name,
    //                 s.to_string()
    //             );
    //         }
    //     }
    // }

    // return Ok(());

    println!("graph Transitions {{");
    println!("  layout = sfdp;");
    println!("  overlap = scale;");
    println!("  K = 0.5;");
    println!("  node [margin=0 fontsize=32 width=0.2 shape=circle style=filled fillcolor=lime];");

    for transition_idx in transition_indices {
        let mut bubble_trans_iter = p.bubbles.iter().enumerate().filter(|(_i, bubble)| {
            bubble
                .transitions
                .iter()
                .find(|p| p.unk0 as usize == transition_idx)
                .is_some()
        });

        let transition = &p.unk10[transition_idx];
        let (bubble_a_idx, bubble_a) = bubble_trans_iter.next().unwrap();
        let Some((bubble_b_idx, bubble_b)) = bubble_trans_iter.next() else {
            println!(
                "  // No second bubble found for transition {}?",
                transition_idx
            );
            continue;
        };
        println!(
            "  {} [ label=\"{}\" ];",
            find_string_fnv(bubble_a.name),
            find_string(&strings, bubble_a.name)
        );
        println!(
            "  {} [ label=\"{}\" ];",
            find_string_fnv(bubble_b.name),
            find_string(&strings, bubble_b.name)
        );
        println!(
            "  {} -- {} [ label=\"{}\" ];",
            find_string_fnv(bubble_a.name),
            find_string_fnv(bubble_b.name),
            find_string_fnv(transition.name)
        );

        orphaned_indices.remove(&bubble_a_idx);
        orphaned_indices.remove(&bubble_b_idx);
    }

    println!("  subgraph Orphaned {{");
    println!(
        "    node [margin=0 fontsize=32 width=0.2 shape=circle style=filled fillcolor=yellow];"
    );
    println!("    edge [color=transparent];");
    for i in &orphaned_indices {
        let v = &p.bubbles[*i];
        println!(
            "    {} [label=\"{}\"];",
            find_string_fnv(v.name),
            find_string(&strings, v.name)
        );
    }

    println!();
    println!(
        "    node [margin=0.5 fontsize=32 width=0.2 shape=box style=filled fillcolor=orange];"
    );
    for i in &orphaned_unk10 {
        let v = &p.unk10[*i];
        println!(
            "    {} [label=\"{}\"];",
            find_string_fnv(v.name),
            find_string(&strings, v.name)
        );
    }

    if !orphaned_unk10.is_empty() {
        println!(
            "    {};",
            orphaned_unk10
                .iter()
                .map(|orphaned_idx| find_string_fnv(p.unk10[*orphaned_idx].name))
                .collect::<Vec<_>>()
                .join(" -- ")
        );
    }
    println!("  }}");

    println!("}}");

    Ok(())
}

const LOAD_STRINGS: &[&str] = &[
    "alleys_a_to_alleys_b",
    "alleys_a",
    "alleys_b_to_alleys_a",
    "alleys_b_to_alleys_c",
    "alleys_b",
    "alleys_c",
    "alleys_to_hydro",
    "alleys_to_sunken",
    "alleys_to_town",
    "basin_dun_a",
    "basin_dun_b",
    "basin_dun_c",
    "basin_to_cove",
    "basin_to_dun_a",
    "basin_to_dun_b_2",
    "basin_to_dun_b",
    "basin_to_dun_c",
    "basin_to_social",
    "basin_to_town",
    "basin_to_treeline",
    "basin",
    "beast",
    "cargo_to_dish",
    "cargo_to_islands",
    "cargo",
    "castle",
    "cove_dun_a",
    "cove_to_dish",
    "cove_to_dun_a_2",
    "cove_to_dun_a",
    "cove_to_gorge_dun_a",
    "cove_to_submerged",
    "cove",
    "dish_dun_a",
    "dish_dun_b",
    "dish_dun_c",
    "dish_to_dun_a",
    "dish_to_dun_b",
    "dish_to_dun_c",
    "dish_to_gorge",
    "dish",
    "edz_social",
    "forest_to_beast",
    "forest_to_shard",
    "forest_to_the_space_between",
    "forest_to_treeline",
    "forest",
    "forge",
    "gorge_dun_a",
    "gorge_to_gorge_dun_a",
    "gorge_to_treeline",
    "gorge_to_tunnels",
    "gorge",
    "helm",
    "hydro_b",
    "hydro_to_town",
    "hydro",
    "islands_dun_a",
    "islands_dun_b",
    "islands_to_dun_a",
    "islands_to_dun_b",
    "islands_to_transport",
    "islands_to_tunnels",
    "islands",
    "ketch_interior_to_islands",
    "mines_to_town",
    "mines",
    "roost_to_transport",
    "roost",
    "shard_a_to_shard_b",
    "shard_a",
    "shard_to_the_space_between",
    "shard",
    "ship_interior",
    "sky_basin",
    "sky_castle",
    "sky_cove",
    "sky_forge",
    "sky_geo_alleys_c",
    "sky_geo_alleys",
    "sky_geo_beast",
    "sky_geo_dun",
    "sky_geo_forest",
    "sky_geo_islands",
    "sky_geo_roost",
    "sky_geo_shard",
    "sky_geo_tango",
    "sky_geo",
    "sky_globals_interior",
    "sky_globals",
    "sky_hydro",
    "sky_social",
    "sky_town",
    "social_to_zz_old_town",
    "submerged_to_helm",
    "submerged",
    "sunken",
    "tango_to_transport",
    "tango",
    "the_space_between_to_forge",
    "the_space_between",
    "town_dun_a",
    "town_dun_b",
    "town_dun_c",
    "town_only",
    "town_to_dun_a",
    "town_to_dun_b",
    "town_to_dun_c",
    "town_to_hydro_b",
    "town",
    "transport",
    "treeline_dun_a",
    "treeline_dun_b_ext",
    "treeline_dun_b",
    "treeline_dun_c",
    "treeline_to_dun_a",
    "treeline_to_dun_b",
    "treeline_to_dun_c",
    "treeline_to_vendor",
    "treeline_vendor",
    "treeline",
    "tunnels_to_zz_crack",
    "tunnels",
    "zz_crack",
    "zz_old_town",
    // cosmo
    "array",
    "array_to_dome_c",
    "array_to_dome_hive",
    "ball",
    "blast",
    "blast_to_ship",
    "breach",
    "breach_sky",
    "breach_to_graveyard",
    "chute",
    "command",
    "crypt",
    "crypt_to_mind_a",
    "delta_n",
    "delta_n_to_delta_s",
    "delta_n_to_dome_a",
    "delta_n_to_dry_sea",
    "delta_s",
    "dome_a",
    "dome_a_to_array",
    "dome_c",
    "dome_c_to_dry_sea",
    "dome_hive",
    "dry_sea",
    "dry_sea_ls_a",
    "dry_sea_to_hive_shore",
    "dry_sea_to_rasputin_vendor",
    "graveyard",
    "graveyard_ls_a",
    "graveyard_to_hangar",
    "graveyard_to_rocketyard",
    "hangar",
    "hangar_sky_geo",
    "hangar_to_delta_s",
    "highway",
    "highway_to_breach",
    "hive_shore",
    "hive_shore_to_mind_a",
    "hoard",
    "mind_a",
    "rasputin",
    "rasputin_to_crypt",
    "rasputin_vendor",
    "rasputin_vendor_to_rasputin",
    "refinery",
    "refinery_to_blast",
    "refinery_to_hive_shore",
    "rocketyard",
    "rocketyard_to_blast",
    "rocketyard_to_command",
    "rocketyard_to_refinery",
    "shanty",
    "shared_terrain_1",
    "ship",
    "ship_to_ball",
    "sky_command_lookout",
    "sky_dome_hive",
    "sky_dry_sea",
    "sky_geo",
    "sky_geo_array",
    "sky_geo_ball",
    "sky_geo_blast",
    "sky_geo_comet",
    "sky_geo_crypt",
    "sky_geo_delta_n",
    "sky_geo_delta_s",
    "sky_geo_dome_a",
    "sky_geo_dome_c",
    "sky_geo_graveyard",
    "sky_geo_highway",
    "sky_geo_hive_shore",
    "sky_geo_mind_a",
    "sky_geo_refinery",
    "sky_geo_rocketyard",
    "sky_geo_ship",
    "sky_globals",
    "sky_globals_array",
    "sky_globals_comet",
    "sky_globals_dome_a",
    "sky_terrain_array",
    "sky_terrain_delta_n",
    "sky_terrain_delta_s",
    "sky_terrain_dry_sea",
    "sky_terrain_graveyard",
    "sky_terrain_highway",
    "sky_terrain_hive_shore",
    "topple",
    // europa
    "approach",
    "assembly",
    "bastion",
    "beam",
    "breach",
    "cine_bastion",
    "collapse",
    "downfall",
    "eminence",
    "factory",
    "flats",
    "frost",
    "frost_b",
    "frost_ls_a",
    "lab",
    "lobby",
    "lobby_to_factory",
    "milk",
    "nexus",
    "overhang",
    "overhang_ls_a",
    "postern",
    "s25_intro",
    "shear",
    "shear_ls_a",
    "sky_geo",
    "sky_globals",
    "streets",
    "zephyr",
    // planet_x
    "ballz",
    "ballz_to_planet_x_exp_a",
    "bubble",
    "cinder_cone",
    "cinder_cone_chamber",
    "cinder_cone_to_cinder_cone_chamber",
    "colony",
    "colony_chamber",
    "colony_dun_a",
    "colony_to_colony_chamber",
    "colony_to_colony_dun_a",
    "colony_to_planet_x_exp_c",
    "colony_to_sunken_cave",
    "drillz",
    "drillz_to_floorz",
    "drillz_to_planet_x_exp_d",
    "floorz",
    "hollows",
    "hollows_to_cinder_cone",
    "hollows_to_planet_x_exp_e",
    "iron_forest",
    "lava_cave",
    "lava_cave_to_planet_x_exp_g",
    "lava_flows",
    "lava_flows_dun_a",
    "lava_flows_to_lava_cave",
    "lava_flows_to_lava_flows_dun_a",
    "lava_flows_to_mini_drillz",
    "lava_flows_to_scorched_plateau",
    "lava_flows_to_well_of_flame",
    "lz",
    "lz_dun_a",
    "lz_sky_geo",
    "lz_sky_plug_geo",
    "lz_to_hollows",
    "lz_to_iron_forest",
    "lz_to_lz_dun_a",
    "lz_to_planet_x_exp_h",
    "lz_to_scorched_plateau",
    "lz_to_waterway",
    "mini_drillz",
    "mini_drillz_to_drillz",
    "mini_drillz_to_planet_x_exp_i",
    "planet_x_exp_a",
    "planet_x_exp_c",
    "planet_x_exp_d",
    "planet_x_exp_e",
    "planet_x_exp_g",
    "planet_x_exp_h",
    "planet_x_exp_i",
    "planet_x_exp_j",
    "planet_x_exp_k",
    "planet_x_exp_l",
    "planet_x_exp_m",
    "planet_x_exp_n",
    "planet_x_exp_o",
    "s24_freeroam",
    "scorched_plateau",
    "scorched_plateau_dun_a",
    "scorched_plateau_to_ballz",
    "scorched_plateau_to_colony",
    "scorched_plateau_to_hollows",
    "scorched_plateau_to_scorched_plateau_dun_a",
    "sky_geo",
    "sky_geo_drillz",
    "sky_geo_planet_x_exp_a",
    "sky_geo_planet_x_exp_c",
    "sky_geo_planet_x_exp_d",
    "sky_geo_planet_x_exp_e",
    "sky_geo_planet_x_exp_g",
    "sky_geo_planet_x_exp_h",
    "sky_geo_planet_x_exp_i",
    "sky_geo_planet_x_exp_j",
    "sky_geo_planet_x_exp_k",
    "sky_geo_planet_x_exp_l",
    "sky_geo_planet_x_exp_m",
    "sky_geo_planet_x_exp_n",
    "sky_geo_planet_x_exp_o",
    "sky_globals",
    "sky_globals_dungeons",
    "sunken_cave",
    "sunken_cave_to_hollows",
    "sunken_cave_to_planet_x_exp_j",
    "sunken_cave_to_well_of_giants",
    "tank",
    "tank_mini",
    "the_mists",
    "the_mists_to_planet_x_exp_k",
    "tomb",
    "tomb_core",
    "trench",
    "trench_to_colony",
    "trench_to_planet_x_exp_l",
    "waterway",
    "waterway_chamber",
    "waterway_dun_a",
    "waterway_to_lava_flows",
    "waterway_to_the_mists",
    "waterway_to_waterway_chamber",
    "waterway_to_waterway_dun_a",
    "waterway_to_well_of_echoes",
    "well_of_echoes",
    "well_of_echoes_to_planet_x_exp_m",
    "well_of_flame",
    "well_of_flame_to_planet_x_exp_n",
    "well_of_giants",
    "well_of_giants_to_planet_x_exp_o",
    "zz_crash",
    // plaguelands
    "blast",
    "blast_to_ship",
    "bluff",
    "bluff_to_bunker",
    "bluff_to_tanker",
    "bluff_to_z_foundry",
    "breach",
    "breach_sky",
    "breach_to_graveyard",
    "bridge",
    "bridge_to_fortress",
    "bunker",
    "bunker_to_bridge",
    "bunker_to_silo",
    "bunker_to_turret",
    "coast",
    "coast_to_tanker",
    "fortress",
    "fortress_to_grotto",
    "fortress_to_tanker",
    "fortress_to_thunderdome",
    "graveyard",
    "graveyard_to_hangar",
    "graveyard_to_rocketyard",
    "grotto",
    "grotto_to_thunderdome",
    "grotto_to_tomb",
    "guardboss",
    "hangar_to_delta_s",
    "highway",
    "highway_to_breach",
    "highway_to_ketch_bay",
    "ketch_bay",
    "refinery",
    "refinery_to_blast",
    "refinery_to_hive_shore",
    "repose",
    "rocketyard",
    "rocketyard_to_command",
    "rocketyard_to_refinery",
    "shared_terrain_1",
    "ship",
    "ship_to_ball",
    "silo",
    "sky_geo",
    "sky_geo_blast",
    "sky_geo_bunker",
    "sky_geo_fortress",
    "sky_geo_fortress_smoke",
    "sky_geo_foundry",
    "sky_geo_graveyard",
    "sky_geo_highway",
    "sky_geo_refinery",
    "sky_geo_reprise",
    "sky_geo_rocketyard",
    "sky_geo_ship",
    "sky_geo_tanker",
    "sky_geo_turret",
    "sky_globals",
    "sky_globals_array",
    "sky_terrain_graveyard",
    "sky_terrain_highway",
    "skybox",
    "tanker",
    "thunderdome",
    "tomb",
    "turret",
    "z_foundry",
    "z_foundry_to_nest",
    "z_guard",
    "z_guard_to_guardboss",
    "z_horizon",
    "z_horizon_to_guard",
    "z_nest",
    "z_nest_to_horizon",
    "z_rocketyard_to_foundry",
    "z_sky_geo_raid",
    "z_sky_geo_raid_foundry",
    "z_sky_globals_raid",
    "z_sky_globals_raid_foundry",
    "blast",
    "bluff",
    "breach",
    "bridge",
    "bunker",
    "coast",
    "fortress",
    "graveyard",
    "grotto",
    "guardboss",
    "highway",
    "ketch_bay",
    "refinery",
    "repose",
    "rocketyard",
    "ship",
    "silo",
    "tanker",
    "thunderdome",
    "tomb",
    "turret",
    "z_foundry",
    "z_guard",
    "z_horizon",
    "z_nest",
];

fn find_string(strings: &StringContainer, hash: u32) -> String {
    if let Some(s) = strings.try_get(hash) {
        if let Some(secondary) = try_find_string_fnv(hash) {
            return format!("{s}\\n[{secondary}]");
        } else {
            return format!("{s}\\n[{hash:08x}]");
        }
    }
    find_string_fnv(hash)
}

fn try_find_string_fnv(hash: u32) -> Option<String> {
    let s = LOAD_STRINGS.iter().find(|s| fnv1(s) == hash);
    s.map(|s| s.to_string())
}

fn find_string_fnv(hash: u32) -> String {
    try_find_string_fnv(hash).unwrap_or_else(|| format!("unk_{hash:08x}"))
}
