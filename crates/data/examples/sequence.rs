use std::{
    fs::File,
    io::{BufRead, BufReader, Cursor, Read, Seek, SeekFrom},
};

use ahash::HashMap;
use alkahest_data::{
    hash::fnv1,
    pattern::SComponent,
    tfx::sequencer::{
        SSequenceNodeBase, SUnk8080816f, SUnk80808179, SUnk808091f1, SUnk808091f1Variant,
    },
};
use chroma_dbg::{ChromaConfig, ChromaDebug};
use tiger_parse::{PackageManagerExt, TigerReadable};
use tiger_pkg::package_manager;

type Wordlist = HashMap<u32, String>;

fn main() -> anyhow::Result<()> {
    alkahest_core::initialize_package_manager(None)?;

    println!("Loading wordlist");

    let mut wordlist = HashMap::default();
    let wordlist_file =
        BufReader::new(File::open("wordlist.txt").expect("Failed to open wordlist file"));

    for line in wordlist_file.lines() {
        let line = line.expect("Failed to read line");
        wordlist.insert(fnv1(&line), line);
    }

    println!("Wordlist loaded");

    // let data = package_manager().read_tag(0x80CF5634)?;
    // let data = package_manager().read_tag(0x80D857A6)?;
    let data = package_manager().read_tag(0x80D91630)?;
    let mut f = Cursor::new(data);
    let component: SComponent = TigerReadable::read_ds(&mut f)?;
    // println!("{:#?}", component);

    if component.unk10.resource_type != 0x80809479 {
        anyhow::bail!(
            "Invalid component type (expected 0x80809479, got {:X})",
            component.unk10.resource_type
        );
    }

    f.seek(SeekFrom::Start(component.unk18.offset))?;

    let globals = SUnk80808179::read_ds(&mut f)?;

    println!("unk158: {}", globals.unk1c8.len());
    println!("unk168: {}", globals.unk1d8.len());
    // println!("unk178: {}", globals.unk1e8.len());
    print_node_recursive(
        &mut f,
        &wordlist,
        (0, 0),
        &globals.unk1c8,
        &globals.unk1d8,
        &globals.unk1f8,
        0,
    );

    // printshit(&globals.unk168, &globals.unk178);

    // TODO: Implement disassembly logic
    Ok(())
}

fn print_node_recursive<F: Read + Seek>(
    f: &mut F,
    wordlist: &Wordlist,
    index: (u16, u16),
    first: &[SUnk808091f1],
    second: &[SUnk808091f1],
    third: &[SUnk8080816f],
    indent: usize,
) {
    let chroma = ChromaConfig {
        inline_array: chroma_dbg::InlineThreshold::MaxLength(1024),
        inline_struct: chroma_dbg::InlineThreshold::MaxLength(1024),
        ..Default::default()
    };

    let i = index.1 as usize;
    // for (i, g) in list.iter().enumerate() {
    let g = if index.0 == 0 {
        &first[i]
    } else if index.0 == 1 {
        &second[i]
    } else {
        panic!("Invalid index: {}", index.0);
    };

    let print_node = |typename: &str, node: &SSequenceNodeBase| {
        println!(
            "node {i}: {typename} [{}] start={} end={} duration={}",
            get_string_fnv(wordlist, node.name),
            chroma.format(&(node.start_time)),
            chroma.format(&(node.start_time + node.duration)),
            chroma.format(&(node.duration))
        );
    };

    let indent_str = "    ".repeat(indent);
    print!("{indent_str}");
    match &*g.unk18 {
        SUnk808091f1Variant::SSequenceGlobalChannel(s) => {
            print_node("GlobalChannel", &s.base);
            // println!("{}", s.dbg_chroma());
            let r = &third[s.other_index as usize];
            println!(
                "{indent_str}  expression 0x{:08X} ({:?})",
                r.unk30,
                get_string_fnv(wordlist, r.unk30)
            );
            // println!("{}", chroma.format(s));
            // // println!("{}", s.dbg_chroma());
            // println!("global_channel[{}]", s.global_channel_index);
            // // println!("{s:?}");
            // println!(
            //     "\tStuff: {:X?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?}",
            //     s.unk00,
            //     s.unk04,
            //     s.unk06,
            //     s.unk08,
            //     s.unk0c,
            //     s.unk10,
            //     s.unk14,
            //     s.unk18,
            //     s.unk1c,
            //     s.other_index
            // );
            // println!("{indent_str}\tBytecode: {:02X?}", &s.bytecode);
            // println!("{indent_str}\tConstants: {:?}", s.bytecode_constants);
            // println!("{indent_str}\tMisc: {}/{}", s.unk50, s.unk58);
            // if let Ok(dis) = sequencer::disassemble(&s.bytecode) {
            //     println!("{indent_str}\tDisassembly:");
            //     for line in dis {
            //         println!("{indent_str}\t\t{line}");
            //     }
            // } else {
            //     println!("{indent_str}\tDisassembly: <failed>");
            // }
            // println!("{indent_str}\tRef: {:X?}", third[s.other_index as usize]);
            // println!();
        }
        // dawn_data::entity::SUnk80809e6Variant::SUnk808093d1(s) => {
        //     println!(
        //         "(???) {i}: [{}] {}",
        //         find_string_for_hash(s.unk0),
        //         chroma.format(s)
        //     );
        //     for c in &s.unk28 {
        //         printshit(f, *c, first, second, third, indent + 1);
        //     }
        // }
        // dawn_data::entity::SUnk80809e6Variant::SUnk808093d3(s) => {
        //     println!(
        //         "(???) {i}: [{}] {}",
        //         find_string_for_hash(s.unk0),
        //         chroma.format(s)
        //     );
        //     for c in &s.unk28 {
        //         printshit(f, *c, first, second, third, indent + 1);
        //     }
        // }
        // dawn_data::entity::SUnk80809e6Variant::SUnk808093cf(s) => {
        //     println!(
        //         "(???) {i}: [{}] {}",
        //         find_string_for_hash(s.unk0),
        //         chroma.format(s)
        //     );
        //     for c in &s.unk28 {
        //         printshit(f, *c, first, second, third, indent + 1);
        //     }
        // }
        // SUnk808091f1Variant::SSequenceEmbeddedParticleSystem(s) => {
        //     print_node("EmbeddedParticleSystem", &s.base);
        // }
        // dawn_data::entity::SUnk80809e6Variant::SSequenceEmbeddedScreenAreaFx(s) => {
        //     print_node("EmbeddedScreenAreaFx", &s.base);
        // }
        SUnk808091f1Variant::SSequenceLight(s) => {
            print_node("Light", &s.base);
        }
        SUnk808091f1Variant::SSequenceLensFlare(s) => {
            print_node("LensFlare", &s.base);
        }
        // dawn_data::entity::SUnk80809e6Variant::SSequenceDelay(s) => {
        //     print_node("Delay", &s.base);
        // }
        SUnk808091f1Variant::SSequenceAudioEvent(s) => {
            print_node(&format!("AudioEvent {}", s.wwise_event.hash32()), &s.base);
        }
        // dawn_data::entity::SUnk80809e6Variant::SSequencePlayerFeedback(s) => {
        //     print_node("PlayerFeedback", &s.base);
        // }
        // dawn_data::entity::SUnk80809e6Variant::SSequenceAreaImpulse(s) => {
        //     print_node("AreaImpulse", &s.base);
        // }
        // dawn_data::entity::SUnk80809e6Variant::SSequenceDamageImpulse(s) => {
        //     print_node("DamageImpulse", &s.base);
        // }
        SUnk808091f1Variant::SUnk808091e3(s) => {
            println!("Parallel {i}: [{}]", get_string_fnv(wordlist, s.base.name));
            for c in &s.children {
                print_node_recursive(f, wordlist, *c, first, second, third, indent + 1);
            }
        }
        SUnk808091f1Variant::SUnk808091df(s) => {
            println!(
                "unk_flow_808091df {i}: [{}]",
                get_string_fnv(wordlist, s.base.name)
            );
            for c in &s.children {
                print_node_recursive(f, wordlist, *c, first, second, third, indent + 1);
            }
        }
        SUnk808091f1Variant::SUnk808091db(s) => {
            println!(
                "unk_flow_808091db {i}: [{}]",
                get_string_fnv(wordlist, s.base.name)
            );
            for c in &s.children {
                print_node_recursive(f, wordlist, *c, first, second, third, indent + 1);
            }
        }
        SUnk808091f1Variant::SUnk808091dd(s) => {
            println!(
                "unk_flow_808091dd {i}: [{}]",
                get_string_fnv(wordlist, s.base.name)
            );
            for c in &s.children {
                print_node_recursive(f, wordlist, *c, first, second, third, indent + 1);
            }
        }
        // dawn_data::entity::SUnk80809e6Variant::SUnk808093d7(s) => {
        //     println!("parallel {i}: [{}]", find_string_for_hash(s.name));
        //     for c in &s.children {
        //         printshit(f, *c, first, second, third, indent + 1);
        //     }
        // }
        // dawn_data::entity::SUnk80809e6Variant::SUnk808093d9(s) => {
        //     println!("serial {i}: [{}]", find_string_for_hash(s.name));
        //     for c in &s.children {
        //         printshit(f, *c, first, second, third, indent + 1);
        //     }
        // }
        SUnk808091f1Variant::Unknown { class, offset } => {
            f.seek(SeekFrom::Start(*offset)).unwrap();
            let hash = u32::read_ds(f).unwrap();

            const ANSI_RED: &str = "\x1b[31m";
            const ANSI_RESET: &str = "\x1b[0m";

            println!(
                "{ANSI_RED}unk {i}: Unknown resource type: {class:08X} @ 0x{offset:X} (name \
                 '{}'){ANSI_RESET}",
                get_string_fnv(wordlist, hash)
            );
        }
    }
}

const CHANNEL_IDS: &[u32] = &[
    413164828, 3564389, 3785521083, 2781995415, 911706265, 3968759000, 2961144874, 359410466,
    1633552291, 2423543701, 90179527, 1252114165, 2556025585, 3626505107, 1302805671, 3981160586,
    2658574034, 1241234657, 3173810152, 2497244380, 2663884264, 3056632075, 1055574819, 2123557681,
    2672605071, 1233537602, 4166210364, 1050803134, 3441842840, 3781913451, 3421710210, 3479309780,
    954006746, 3454161171, 1067185869, 1659130926, 2045960965, 2492950735, 3867526455, 104698615,
    3000034716, 3835805536, 3651328163, 2243399393, 3800300512, 1597736081, 743670141, 743670142,
    743670143, 743670136, 743670137, 743670138, 743670139, 743670132, 743670133, 760447727,
    760447726, 760447725, 760447724, 760447723, 760447722, 760447721, 760447720, 760447719,
    760447718, 777225282, 777225283, 777225280, 777225281, 777225286, 777225287, 777225284,
    777225285, 777225290, 777225291, 794002997, 794002996, 794002999, 3638890901, 4284370545,
    282035500, 95034066, 1338215099, 61911189, 1714731569, 2088734839, 1383168257, 4060811251,
    1614739564, 3439522578, 1962697412, 3924713116, 3437944730, 4264009506, 1007758803, 1007758800,
    547401102, 294761227, 1549245946, 953731187, 3231367590, 1847767646, 2086794555, 2121305497,
    2555718632, 3828643774, 4033942491, 4033942488, 4033942489, 4222214337, 2093588672, 3970459769,
    3970459770, 3970459771, 3970459772, 3970459773, 3970459774, 3970459775, 3970459760, 4074476269,
    181827042, 3870210540, 1129680604, 3137538898, 3480267900, 1193158898, 3862473451, 357608460,
    1942337203, 4081239241, 308616162, 3347321793, 2404723577, 1305631816, 1337804320, 1061091340,
    1150719031, 125854326, 1271106181, 1476445940, 1476445943, 1476445942, 1476445937, 1476445936,
    1476445939, 1476445938, 1476445949, 1476445948, 1459668226, 1459668227, 1459668224, 3076743815,
    3732873324,
];

fn get_string_fnv(wordlist: &Wordlist, hash: u32) -> String {
    wordlist
        .get(&hash)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("0x{hash:08X}"))
}
