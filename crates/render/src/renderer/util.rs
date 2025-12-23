use alkahest_data::tfx::{PipelineState, PrimitiveType};

use super::Renderer;
use crate::{
    cmd_event_span, gpu::command_list::CommandList, renderer::surface::SurfaceHandle,
    tfx::technique::Technique,
};

impl Renderer {
    pub fn execute_global_pipeline(&self, cmd: &mut CommandList, pipeline: &Technique, name: &str) {
        cmd_event_span!(cmd, &format!("[{name}]"));
        if let Err(e) = pipeline.bind(cmd) {
            error!("Failed to run {name}: {e}");
            return;
        }

        cmd.flush_states();
        cmd.set_input_topology(PrimitiveType::TriangleStrip);

        cmd.draw(4, 0);
    }

    pub fn blit_surface(
        &self,
        cmd: &mut CommandList,
        src: SurfaceHandle,
        dst: SurfaceHandle,
        linear: bool,
        name: &str,
    ) {
        self.blit_srv(
            cmd,
            &self.surfaces().get(src).srv,
            &self.surfaces().get(dst).rtv,
            linear,
            name,
        );
    }

    pub fn blit_srv<'a>(
        &self,
        cmd: &mut CommandList,
        srv: impl Into<Option<&'a d3d11::ShaderResourceView>>,
        rtv: impl Into<Option<&'a d3d11::RenderTargetView>>,
        linear: bool,
        name: &str,
    ) {
        cmd_event_span!(cmd, &format!("blit_srv [{name}]"));

        cmd.vertex_set_shader(&self.common.blit_vs);
        if linear {
            cmd.pixel_set_shader(&self.common.blit_ps_linear);
        } else {
            cmd.pixel_set_shader(&self.common.blit_ps);
        }

        {
            cmd.pixel_set_shader_resources(0, &[srv.into()]);
            cmd.output_merger_set_render_targets(&[rtv.into()], None);
        }

        cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
        cmd.flush_states();
        cmd.set_input_topology(PrimitiveType::TriangleStrip);

        cmd.draw(4, 0);
    }
}
