use alkahest_data::geometry::EPrimitiveType;
use bevy_ecs::component::Component;
use glam::Mat4;
use windows::Win32::Graphics::{
    Direct3D11::{ID3D11Buffer, ID3D11DepthStencilState},
    Dxgi::Common::DXGI_FORMAT_R16_UINT,
};

use crate::{gpu_event, renderer::Renderer, tfx::externs::DeferredUberLight};

#[derive(Component)]
pub struct UberLightRenderer {
    pub projection_matrix: Mat4,
    pub parameters: DeferredUberLight,

    depth_state: ID3D11DepthStencilState,
    vb_cube: ID3D11Buffer,
    ib_cube: ID3D11Buffer,
    cube_index_count: u32,
}

impl UberLightRenderer {
    pub fn draw(&self, renderer: &Renderer) {
        gpu_event!(renderer.gpu, "deferred_uber_light");
        {
            renderer.data.lock().externs.deferred_uber_light = Some(self.parameters.clone());
        }

        unsafe {
            renderer
                .gpu
                .context()
                .OMSetDepthStencilState(Some(&self.depth_state), 0);

            // Layout 1
            //  - float3 v0 : POSITION0, // Format DXGI_FORMAT_R32G32B32_FLOAT size 12
            renderer.gpu.set_input_layout(1);
            renderer.gpu.set_blend_state(8);
            renderer.gpu.context().IASetVertexBuffers(
                0,
                1,
                Some([Some(self.vb_cube.clone())].as_ptr()),
                Some([12].as_ptr()),
                Some(&0),
            );
            let tech = &renderer.render_globals.pipelines.deferred_uber_light;
            tech.bind(renderer).expect("Failed to bind technique");

            renderer
                .gpu
                .context()
                .IASetIndexBuffer(Some(&self.ib_cube), DXGI_FORMAT_R16_UINT, 0);

            renderer.gpu.set_input_topology(EPrimitiveType::Triangles);

            renderer
                .gpu
                .context()
                .DrawIndexed(self.cube_index_count, 0, 0);
        }
    }
}
