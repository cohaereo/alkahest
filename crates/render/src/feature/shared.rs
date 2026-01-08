use tiger_pkg::TagHash;

use crate::{
    Renderer,
    asset::{index_buffer::IndexBuffer, vertex_buffer::VertexBuffer},
    gpu::command_list::CommandList,
};

pub(super) struct ModelBuffers {
    pub vertex0_buffer: VertexBuffer,
    pub vertex1_buffer: Option<VertexBuffer>,
    pub color_buffer: Option<VertexBuffer>,
    pub index_buffer: IndexBuffer,
}

impl ModelBuffers {
    pub fn load(
        vertex0_buffer: TagHash,
        vertex1_buffer: TagHash,
        color_buffer: TagHash,
        index_buffer: TagHash,
    ) -> anyhow::Result<Self> {
        let gpu = &Renderer::instance().gpu;
        Ok(Self {
            vertex0_buffer: VertexBuffer::load(gpu, vertex0_buffer)?,
            vertex1_buffer: if vertex1_buffer.is_some() {
                Some(VertexBuffer::load(gpu, vertex1_buffer)?)
            } else {
                None
            },
            color_buffer: if color_buffer.is_some() {
                Some(VertexBuffer::load(gpu, color_buffer)?)
            } else {
                None
            },
            index_buffer: IndexBuffer::load(gpu, index_buffer)?,
        })
    }

    #[profiling::function]
    pub fn bind(&self, cmd: &mut CommandList) -> Option<()> {
        self.index_buffer.bind(cmd);

        let vertex1 = self.vertex1_buffer.as_ref().unwrap_or(&self.vertex0_buffer); // or a dummy buffer

        cmd.input_assembler_set_vertex_buffers(
            0,
            &[Some(&self.vertex0_buffer.buffer), Some(&vertex1.buffer)],
            Some(&[self.vertex0_buffer.stride as _, vertex1.stride as _]),
            Some(&[0, 0]),
        )
        .ok()?;

        if let Some(color_buffer) = &self.color_buffer {
            cmd.vertex_set_shader_resources(0, &[color_buffer.srv.as_ref()]);
        } else {
            cmd.vertex_set_shader_resources(
                0,
                &[Renderer::instance()
                    .common
                    .vertex_color_fallback
                    .srv
                    .as_ref()],
            );
        }

        Some(())
    }
}
