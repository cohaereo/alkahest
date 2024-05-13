use alkahest_data::{geometry::EPrimitiveType, technique::StateSelection};

use crate::{
    ecs::{map::CubemapVolume, Scene},
    renderer::Renderer,
    tfx::{externs, globals::CubemapShape},
};

pub fn draw_cubemap_system(renderer: &Renderer, scene: &Scene) {
    {
        renderer.data.lock().externs.cubemaps = Some(externs::Cubemaps {
            temp_ao: renderer.gpu.white_texture.view.clone().into(),
        });
    }

    // for (_e, cubemap) in scene.query::<&CubemapVolume>().iter() {
    //     let alpha = false;
    //     let probes = cubemap.voxel_diffuse.is_some();
    //     let relighting = false;
    //
    //     let pipeline = renderer
    //         .render_globals
    //         .pipelines
    //         .get_specialized_cubemap_pipeline(CubemapShape::Cube, alpha, probes, relighting);
    //
    //     if let Err(e) = pipeline.bind(renderer) {
    //         error!(
    //             "Failed to run cubemap pipeline (alpha={alpha}, probes={probes}, \
    //              relighting={relighting}): {e:?}"
    //         );
    //         return;
    //     }
    // }
}
