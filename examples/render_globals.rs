use alkahest_data::tfx::{STechnique, render_globals::SRenderGlobals};
use anyhow::Context;
use tiger_parse::PackageManagerExt;
use tiger_pkg::package_manager;

fn main() -> anyhow::Result<()> {
    alkahest_core::initialize_package_manager(std::env::args().nth(1).as_deref())?;

    let data: SRenderGlobals = package_manager().read_named_tag_struct("render_globals")?;
    let globs = &data.unk8.first().context("No render globals found")?.unk8.0;

    for pipeline in &globs.pipelines {
        println!("{} > techniques/{}.tfx", pipeline.technique, *pipeline.name,);
        let Ok(technique) = package_manager().read_tag_struct::<STechnique>(pipeline.technique)
        else {
            continue;
        };

        for (stage, shader) in technique.all_valid_shaders() {
            let entry = package_manager()
                .get_entry(shader.shader)
                .context("Failed to get shader entry")?;
            let bytecode = package_manager().read_tag(entry.reference)?;
            std::fs::write(
                format!("techniques/{}_{:?}.cso", *pipeline.name, stage),
                &bytecode,
            )?;
        }
    }

    Ok(())
}
