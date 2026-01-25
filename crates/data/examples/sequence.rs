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
use chroma_dbg::ChromaConfig;
use tiger_parse::TigerReadable;
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

fn get_string_fnv(wordlist: &Wordlist, hash: u32) -> String {
    wordlist
        .get(&hash)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("0x{hash:08X}"))
}
