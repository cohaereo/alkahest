use crate::entity::VertexBufferHeader;
use crate::packages::package_manager;
use crate::render::renderer::Renderer;
use crate::render::vertex_layout;
use destiny_pkg::TagHash;
use itertools::Itertools;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use tiger_parse::PackageManagerExt;

/// Returns the hash of the resulting input layout
pub fn load_vertex_buffers(
    renderer: &Renderer,
    material: TagHash,
    buffers: &[TagHash],
) -> anyhow::Result<u64> {
    if !material.is_some() {
        anyhow::bail!("Invalid material {material}");
    }

    let mut buffer_strides = vec![];
    for b in buffers {
        if b.is_some() {
            let vertex_header: VertexBufferHeader = package_manager().read_tag_struct(*b).unwrap();
            buffer_strides.push(vertex_header.stride as usize);
        } else {
            buffer_strides.push(0);
        }
    }

    let buffer_offsets = buffer_strides
        .iter()
        .scan(0, |offset, &stride| {
            let current_offset = *offset;
            *offset += stride;
            Some(current_offset)
        })
        .collect_vec();

    let (material_vshader, material_pshader) = {
        renderer.render_data.load_technique(renderer, material);

        let render_data = renderer.render_data.data();
        let mat = &render_data.techniques[&material];

        (
            mat.stage_vertex.shader.shader,
            mat.stage_pixel.shader.shader,
        )
    };

    let vshader = renderer
        .render_data
        .load_vshader(&renderer.dcs, material_vshader)
        .unwrap();
    renderer
        .render_data
        .load_pshader(&renderer.dcs, material_pshader);

    let mut new_input_layout = vshader.1.clone();
    let mut layout_offset = 0;
    for element in new_input_layout.iter_mut() {
        element.input_slot = buffer_offsets
            .iter()
            .positions(|v| layout_offset >= *v)
            .last()
            .unwrap_or(buffer_offsets.len() - 1) as u32;

        layout_offset += element.format.bpp() / 8;
    }

    let mut s = DefaultHasher::new();
    new_input_layout.hash(&mut s);
    let hash = s.finish();

    // let layout_string = new_input_layout
    //     .iter()
    //     .filter(|e| !e.semantic_type.is_system_value())
    //     .enumerate()
    //     .map(|(i, e)| {
    //         format!(
    //             "\t{}{} v{i} : {}{} (slot {})",
    //             e.component_type,
    //             e.component_count,
    //             unsafe { e.semantic_type.to_pcstr().display() },
    //             e.semantic_index,
    //             e.input_slot
    //         )
    //     })
    //     .join("\n");
    // println!("0x{hash:x}, {} buffers\n{layout_string}\n", buffers.len());

    if !renderer
        .render_data
        .data()
        .input_layouts
        .contains_key(&hash)
    {
        let layout_converted = vertex_layout::build_input_layout(&new_input_layout);

        // println!("Input offsets: {:?}", buffer_offsets);
        // println!("Input strides: {:?}", buffer_strides);
        // println!(
        //     "Converted layout offsets: {:?}",
        //     layout_converted
        //         .iter()
        //         .map(|v| (
        //             v.InputSlot,
        //             v.AlignedByteOffset,
        //             DxgiFormat::try_from(v.Format.0).unwrap().bpp() / 8
        //         ))
        //         .collect_vec()
        // );

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

        renderer
            .render_data
            .data_mut()
            .input_layouts
            .insert(hash, input_layout);
    }

    Ok(hash)
}
