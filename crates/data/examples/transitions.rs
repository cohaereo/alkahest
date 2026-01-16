use ahash::HashSet;
use alkahest_data::{activity::transition::S808096BF, hash::fnv1, strings::StringContainer};
use chroma_dbg::ChromaDebug;
use tiger_parse::PackageManagerExt;
use tiger_pkg::package_manager;

fn main() -> anyhow::Result<()> {
    alkahest_core::initialize_package_manager(None)?;

    let strings = StringContainer::load(0x80A687EC)?;
    let p: S808096BF = package_manager().read_tag_struct(0x80BB313E)?;
    println!("{}", p.dbg_chroma());
    let mut transition_indices = HashSet::default();
    let mut orphaned_indices = HashSet::default();
    let mut orphaned_unk10 = HashSet::default();

    for (i, _) in p.unk10.iter().enumerate() {
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

    // for (i, v) in p.unk10.iter().enumerate() {
    //     println!("{i}: {} {}", find_string(&strings, v.name), v.dbg_chroma());
    // }

    // return Ok(());

    println!("graph Transitions {{");
    println!("  layout = sfdp;");
    println!("  overlap = scalexy;");
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

        let (bubble_a_idx, bubble_a) = bubble_trans_iter.next().unwrap();
        let (bubble_b_idx, bubble_b) = bubble_trans_iter.next().unwrap();
        let transition = &p.unk10[transition_idx];
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
    println!("    node [margin=0 fontsize=32 width=0.2 shape=circle style=filled fillcolor=red];");
    println!("    edge [color=transparent];");
    for i in &orphaned_indices {
        let v = &p.bubbles[*i];
        println!(
            "    {} [label=\"{}\"];",
            find_string_fnv(v.name),
            find_string(&strings, v.name)
        );
    }

    println!(
        "    {};",
        orphaned_indices
            .iter()
            .map(|orphaned_idx| find_string_fnv(p.bubbles[*orphaned_idx].name))
            .collect::<Vec<_>>()
            .join(" -- ")
    );
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

    println!(
        "    {};",
        orphaned_unk10
            .iter()
            .map(|orphaned_idx| find_string_fnv(p.unk10[*orphaned_idx].name))
            .collect::<Vec<_>>()
            .join(" -- ")
    );
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
];

fn find_string(strings: &StringContainer, hash: u32) -> String {
    if let Some(s) = strings.try_get(hash) {
        if let Some(secondary) = try_find_string_fnv(hash) {
            return format!("{s}\\n[{secondary}]");
        } else {
            return s;
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
