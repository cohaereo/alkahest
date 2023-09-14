use crate::material::Material;
use crate::packages::package_manager;
use crate::render::renderer::Renderer;
use crate::render::{self, vertex_layout};
use destiny_pkg::TagHash;
use itertools::Itertools;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Returns the hash of the resulting input layout
pub fn load_vertex_buffers(
    renderer: &Renderer,
    material: TagHash,
    buffers: &[TagHash],
) -> anyhow::Result<u64> {
    if !material.is_valid() {
        anyhow::bail!("Invalid material {material}");
    }

    let (material_vshader, material_pshader) = {
        renderer.render_data.load_material(renderer, material);

        let render_data = renderer.render_data.data();
        let mat = &render_data.materials[&material];

        (mat.vertex_shader, mat.pixel_shader)
    };

    let vshader = renderer
        .render_data
        .load_vshader(&renderer.dcs, material_vshader)
        .unwrap();
    renderer
        .render_data
        .load_pshader(&renderer.dcs, material_pshader);

    let base_input_layout = vshader.1.clone();

    let mut s = DefaultHasher::new();
    base_input_layout.hash(&mut s);
    let hash = s.finish();

    // let layout_string = base_input_layout
    //     .iter()
    //     .filter(|e| !e.semantic_type.is_system_value())
    //     .enumerate()
    //     .map(|(i, e)| {
    //         format!(
    //             "\t{}{} v{i} : {}{}",
    //             e.component_type,
    //             e.component_count,
    //             unsafe { e.semantic_type.to_pcstr().display() },
    //             e.semantic_index
    //         )
    //     })
    //     .join("\n");
    // println!("0x{hash:x}\n{layout_string}\n");

    if !renderer
        .render_data
        .data()
        .input_layouts
        .contains_key(&hash)
    {
        let layout_converted = vertex_layout::build_input_layout(&base_input_layout);

        let input_layout = {
            let vs_data = &renderer.render_data.data().vshaders[&material_vshader].2;
            unsafe {
                renderer
                    .dcs
                    .device
                    .CreateInputLayout(&layout_converted, vs_data)
            }
            .unwrap()
        };

        // let layout_string = base_input_layout
        //     .iter()
        //     .filter(|e| !e.semantic_type.is_system_value())
        //     .enumerate()
        //     .map(|(i, e)| {
        //         format!(
        //             "\t{}{} v{i} : {}{}",
        //             e.component_type,
        //             e.component_count,
        //             unsafe { e.semantic_type.to_pcstr().display() },
        //             e.semantic_index
        //         )
        //     })
        //     .join("\n");

        // info!(
        //     "Loaded vertex layout for VS {:?}, layout:\n{}\n",
        //     material_vshader, layout_string
        // );

        renderer
            .render_data
            .data_mut()
            .input_layouts
            .insert(hash, input_layout);
    }

    Ok(hash)
}
