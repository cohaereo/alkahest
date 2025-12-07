use alkahest_data::tfx::{PipelineState, ShaderStage};

use super::command_list::CommandList;
use crate::gpu::command_list::ContextExt;

pub struct GpuState {
    tfx_state: PipelineState,
    current_blend_state: usize,
    current_depth_state: usize,
    current_rasterizer_state: usize,
    current_depth_bias: usize,
    current_input_layout: usize,
    current_input_topology: usize,

    state_vs: GpuStageState,
    state_ps: GpuStageState,
    // state_cs: GpuStageState,
    viewports: [d3d11::Viewport; 8],
    rtvs: [Option<d3d11::RenderTargetView>; 8],
    dsv: Option<d3d11::DepthStencilView>,
}

/// Represents the state of a GPU stage (vertex, fragment, compute, etc.)
struct GpuStageState {
    cbuffers: [Option<d3d11::Buffer>; d3d11::DeviceContext::CONSTANT_BUFFER_SLOT_COUNT],
    samplers: [Option<d3d11::SamplerState>; d3d11::DeviceContext::SAMPLER_SLOT_COUNT],
    // srvs: [Option<d3d11::ShaderResourceView>; d3d11::DeviceContext::SHADER_RESOURCE_SLOT_COUNT],
}

impl GpuState {
    #[profiling::function]
    pub fn backup(cmd: &CommandList) -> Self {
        let (rtvs, dsv) = cmd.output_merger_get_render_targets();
        GpuState {
            tfx_state: cmd.state,
            current_blend_state: cmd.current_blend_state,
            current_depth_state: cmd.current_depth_state,
            current_rasterizer_state: cmd.current_rasterizer_state,
            current_depth_bias: cmd.current_depth_bias,
            current_input_layout: cmd.current_input_layout,
            current_input_topology: cmd.current_input_topology,
            state_vs: GpuStageState {
                cbuffers: cmd.vertex_get_constant_buffers(),
                samplers: cmd.vertex_get_samplers(),
            },
            state_ps: GpuStageState {
                cbuffers: cmd.pixel_get_constant_buffers(),
                samplers: cmd.pixel_get_samplers(),
            },
            viewports: cmd.rasterizer_get_viewports(),
            rtvs,
            dsv,
        }
    }

    #[profiling::function]
    pub fn restore(&self, cmd: &mut CommandList) {
        cmd.state = self.tfx_state;
        cmd.current_blend_state = self.current_blend_state;
        cmd.current_depth_state = self.current_depth_state;
        cmd.current_rasterizer_state = self.current_rasterizer_state;
        cmd.current_depth_bias = self.current_depth_bias;
        cmd.current_input_layout = self.current_input_layout;
        cmd.current_input_topology = self.current_input_topology;
        {
            profiling::scope!("restore_vs");
            cmd.set_sampler(ShaderStage::Vertex, 0, &self.state_vs.samplers[0]);
            cmd.set_sampler(ShaderStage::Vertex, 1, &self.state_vs.samplers[1]);
            cmd.set_constant_buffer(ShaderStage::Vertex, 12, &self.state_vs.cbuffers[12]);
            cmd.set_constant_buffer(ShaderStage::Vertex, 13, &self.state_vs.cbuffers[13]);
        }

        {
            profiling::scope!("restore_ps");
            cmd.set_sampler(ShaderStage::Pixel, 0, &self.state_ps.samplers[0]);
            cmd.set_sampler(ShaderStage::Pixel, 1, &self.state_ps.samplers[1]);
            cmd.set_constant_buffer(ShaderStage::Pixel, 12, &self.state_ps.cbuffers[12]);
            cmd.set_constant_buffer(ShaderStage::Pixel, 13, &self.state_ps.cbuffers[13]);
        }

        cmd.rasterizer_set_viewports(&self.viewports[0..4]);
        cmd.output_merger_set_render_targets(
            &[
                self.rtvs[0].as_ref(),
                self.rtvs[1].as_ref(),
                self.rtvs[2].as_ref(),
                self.rtvs[3].as_ref(),
            ],
            self.dsv.as_ref(),
        );
        cmd.flush_states();
    }
}
