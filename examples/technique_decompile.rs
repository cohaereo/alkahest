use std::str::FromStr;

use alkahest_data::tfx::STechnique;
use alkahest_render::tfx::expression_vm::{self, decompiler::DecompilerState};
use anyhow::Context;
use itertools::Itertools;
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

fn main() -> anyhow::Result<()> {
    let Some(hash) = std::env::args().nth(1) else {
        anyhow::bail!("Usage: technique_decompile <technique tag>");
    };

    let Ok(hash) = TagHash::from_str(&hash) else {
        anyhow::bail!("Invalid technique tag hash: {}", hash);
    };

    alkahest_core::initialize_package_manager(None)?;

    let tech: STechnique = package_manager()
        .read_tag_struct(hash)
        .context("Failed to read/parse tag")?;

    for (stage, shader) in tech.all_valid_shaders() {
        println!("// Stage: {stage:?}");
        println!("\t// Disassembly:");
        match expression_vm::disassemble(&shader.constants.bytecode) {
            Ok(lines) => {
                for line in lines {
                    println!("\t{line}");
                }
            }
            Err(e) => {
                println!("\t// Failed to disassemble expression: {e}");
                continue;
            }
        }

        println!("\t// Decompiled assignments:");
        match DecompilerState::new(&shader.constants.bytecode)
            .with_ansi(false)
            .evaluate(&shader.constants.bytecode_constants)
        {
            Ok(o) => {
                println!("\t{}", o.pretty_print().split("\n").join("\n\t"));
            }
            Err(e) => println!("\t// Failed to decompile expression: {e}"),
        };
    }

    Ok(())
}
