use alkahest_data::tfx::PrimitiveType;

use super::Renderer;
use crate::{cmd_event_span, gpu::command_list::CommandList, tfx::technique::Technique};

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
}
