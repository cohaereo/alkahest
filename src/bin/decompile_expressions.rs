use alkahest_data::tfx::{render_globals::SRenderGlobals, scope::SScope};
use alkahest_render::tfx::expression_vm::{self, decompiler::DecompilerState};
use anyhow::Context;
use itertools::Itertools;
use tiger_parse::PackageManagerExt;
use tiger_pkg::package_manager;

fn main() -> anyhow::Result<()> {
    alkahest_core::initialize_package_manager(std::env::args().nth(1).as_deref())?;

    let data: SRenderGlobals = package_manager().read_named_tag_struct("render_globals")?;
    let globs = &data.unk8.first().context("No render globals found")?.unk8.0;

    for scope in globs.scopes.iter() {
        println!("Scope: {}", scope.name.to_string());
        let scope: SScope = package_manager().read_tag_struct(scope.scope)?;
        for (stage, shader_stage) in scope
            .iter_stages()
            .filter(|(stage, _)| stage.constants.constant_buffer_slot != 0)
        {
            println!(
                "  Stage: {:?}, CB Slot: {}",
                shader_stage, stage.constants.constant_buffer_slot
            );

            if stage.constants.bytecode.is_empty() {
                println!("    // No bytecode present.");
                continue;
            }

            println!("    // Disassembly:");
            match expression_vm::disassemble(&stage.constants.bytecode) {
                Ok(lines) => {
                    for line in lines {
                        println!("    {line}");
                    }
                }
                Err(e) => {
                    println!("    // Failed to disassemble expression: {e}");
                    continue;
                }
            }

            println!("    // Decompiled assignments:");
            match DecompilerState::new(&stage.constants.bytecode)
                .with_ansi(true)
                .evaluate(&stage.constants.bytecode_constants)
            {
                Ok(o) => {
                    println!("    {}", o.pretty_print().split("\n").join("\n    "));
                }
                Err(e) => println!("    // Failed to decompile expression: {e}"),
            };
        }
    }

    Ok(())
}
