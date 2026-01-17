use std::collections::HashSet;

use alkahest_data::tfx::{ExternIndex, SDynamicConstants, STechnique, scope::SScope};
use alkahest_render::tfx::expression_vm::opcodes::{Opcode, OpcodeIterator, pascal_to_snake};
use itertools::Itertools;
use tiger_parse::{PackageManagerExt, TigerReadable};
use tiger_pkg::package_manager;

fn main() -> anyhow::Result<()> {
    alkahest_core::initialize_package_manager(std::env::args().nth(1).as_deref())?;

    #[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
    pub enum ExternFieldType {
        Float,
        Vec4,
        Mat4,
        U32,
        Texture,
        Uav,
    }

    let mut fields: HashSet<(ExternIndex, ExternFieldType, usize)> = Default::default();

    let mut process_constants = |dc: &SDynamicConstants| {
        for (op, args) in OpcodeIterator::new(&dc.bytecode) {
            if !matches!(
                op,
                Opcode::PushExternInputFloat
                    | Opcode::PushExternInputVec4
                    | Opcode::PushExternInputMat4
                    | Opcode::PushExternInputTextureView
                    | Opcode::PushExternInputU32
                    | Opcode::PushExternInputUav
            ) {
                continue;
            }

            let extern_ = ExternIndex::try_from(args[0]).unwrap();
            let offset_raw = args[1];

            match op {
                Opcode::PushExternInputFloat => {
                    fields.insert((extern_, ExternFieldType::Float, offset_raw as usize * 4));
                }
                Opcode::PushExternInputVec4 => {
                    fields.insert((extern_, ExternFieldType::Vec4, offset_raw as usize * 16));
                }
                Opcode::PushExternInputMat4 => {
                    fields.insert((extern_, ExternFieldType::Mat4, offset_raw as usize * 16));
                }
                Opcode::PushExternInputTextureView => {
                    fields.insert((extern_, ExternFieldType::Texture, offset_raw as usize * 8));
                }
                Opcode::PushExternInputU32 => {
                    fields.insert((extern_, ExternFieldType::U32, offset_raw as usize * 4));
                }
                Opcode::PushExternInputUav => {
                    fields.insert((extern_, ExternFieldType::Uav, offset_raw as usize * 8));
                }
                _ => {}
            }
        }
    };

    for (t, _) in package_manager()
        .get_all_by_reference(SScope::ID.unwrap())
        .into_iter()
    {
        let Ok(scope): tiger_parse::Result<SScope> = package_manager().read_tag_struct(t) else {
            continue;
        };
        for (s, _) in scope.iter_stages() {
            process_constants(&s.constants);
        }
    }

    for (t, _) in package_manager()
        .get_all_by_reference(STechnique::ID.unwrap())
        .into_iter()
    {
        let Ok(technique): tiger_parse::Result<STechnique> = package_manager().read_tag_struct(t)
        else {
            continue;
        };
        for (_, s) in technique.all_shaders() {
            process_constants(&s.constants);
        }
    }

    for ext in (0..ExternIndex::COUNT).map(|v| ExternIndex::try_from(v as u8).unwrap()) {
        let mut sfields = fields
            .iter()
            .filter(|(e, _, _)| *e == ext)
            .map(|(_, a, b)| (*a, *b))
            .collect_vec();

        sfields.sort_by_key(|(_, offset)| *offset);

        if sfields.is_empty() {
            continue;
        }

        println!("extern_struct! {{");
        println!(
            "\tstruct {ext:?}(\"{}\") {{",
            pascal_to_snake(&format!("{ext:?}"))
        );

        for (ty, offset) in sfields {
            let ty_str = match ty {
                ExternFieldType::Float => "f32",
                ExternFieldType::Vec4 => "Vec4",
                ExternFieldType::Mat4 => "Mat4",
                ExternFieldType::U32 => "u32",
                ExternFieldType::Texture => "TextureView",
                ExternFieldType::Uav => "UnorderedAccessView",
            };

            println!("\t\t0x{offset:02X} => unk{offset:02x}: {ty_str},");
        }

        println!("\t}}");
        println!("}}\n");
    }

    Ok(())
}
