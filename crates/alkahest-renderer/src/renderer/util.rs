use alkahest_data::{geometry::EPrimitiveType, technique::StateSelection};

use super::Renderer;
use crate::tfx::technique::Technique;

impl Renderer {
    pub(super) fn execute_global_pipeline(&self, pipeline: &Technique, name: &str) {
        if let Err(e) = pipeline.bind(self) {
            error!("Failed to run {name}: {e}");
            return;
        }

        self.gpu.flush_states();
        self.gpu.set_input_topology(EPrimitiveType::TriangleStrip);

        unsafe {
            self.gpu.context().Draw(4, 0);
        }
    }
}
