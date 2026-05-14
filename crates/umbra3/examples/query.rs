use std::time::Instant;

use anyhow::Context;
use glam::{Mat4, vec3};
use umbra::{CameraTransform, OcclusionBuffer, Query, Tome, Visibility};

fn main() -> anyhow::Result<()> {
    let tome_filename = std::env::args().nth(1).context("Missing tome argument")?;
    let tome_data = std::fs::read(tome_filename)?;

    let tome = Tome::load_from_buffer(&tome_data);

    let position = vec3(0.0, 0.0, 5.0);
    let world_to_ndc = Mat4::perspective_rh(90f32.to_radians(), 16.0 / 9.0, 0.1, 5000.0)
        * Mat4::look_at_lh(position, vec3(0.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0));

    let mut query = Query::new(&tome);

    let mut vis = Visibility::default();
    let mut ob = OcclusionBuffer::default();
    vis.set_output_buffer(&mut ob);

    query.query_portal_visibility(
        0,
        &vis,
        &CameraTransform::new(world_to_ndc.to_cols_array_2d(), position.to_array()),
        0.0,
        -1.0,
        0,
        1,
        0,
    );

    let desc = ob.get_desc();
    let mut output = vec![64; desc.width as usize * desc.height as usize];

    ob.get_buffer(&mut output, &desc);
    std::fs::write("img.data", output)?;

    Ok(())
}
