use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use alkahest_data::tfx::{
    FeatureRendererSubscription, PipelineState, PrimitiveType, RenderStage, ShaderStage,
};
use anyhow::Context;
use crossbeam::atomic::AtomicCell;
use d3d11::dxgi;
use glam::UVec2;
use parking_lot::Mutex;

use super::Renderer;
use crate::{
    Gpu, cmd_event_span,
    gpu::{
        command_list::CommandList,
        gpu_buffer::{GpuBuffer, GpuBufferDesc},
    },
    renderer::{
        submit::gbuffer::CoordParams,
        surface::{SizeRelativity, SurfaceDesc, SurfaceHandle, SurfaceProxy, Surfaces},
    },
    tfx::view::{MainView, View},
};

pub struct PickBuffers {
    pub depth: SurfaceHandle,
    pub depth_proxy: Mutex<SurfaceProxy>,
    pub pickbuffer: SurfaceHandle,
    pub pickbuffer_proxy: Mutex<SurfaceProxy>,
    pub pickbuffer_debug: SurfaceHandle,
    pub pickbuffer_sample: GpuBuffer<u32>,

    selection_request: AtomicCell<Option<(u32, u32)>>,
    selection_ready: AtomicBool,
    /// The entity that's already selected. Will not be drawn into the pickbuffer
    pub selected_entity: Option<u32>,
}

impl PickBuffers {
    pub fn create(
        gpu: &Gpu,
        surfaces: &Surfaces,
        base_resolution: (u32, u32),
    ) -> anyhow::Result<Self> {
        // let depth = DepthState::create(gpu, base_resolution, "gbuffer_depth")?;
        let depth = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("pickbuffer_depth", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R32g8x24Typeless)
                .depth_format(dxgi::Format::D32FloatS8x24Uint)
                .build(),
        )?;

        let pickbuffer_sample = GpuBuffer::new(
            &gpu.device,
            GpuBufferDesc::builder("pickbuffer_sample").build(),
        )?;

        let desc_pickbuffer =
            SurfaceDesc::builder("pickbuffer", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R32Typeless)
                .view_format(dxgi::Format::R32Uint)
                .build();
        let pickbuffer = surfaces.create_surface(base_resolution, desc_pickbuffer.clone())?;

        let desc_pickbuffer_debug =
            SurfaceDesc::builder("pickbuffer_debug", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R10g10b10a2Typeless)
                .view_format(dxgi::Format::R10g10b10a2Unorm)
                .build();
        let pickbuffer_debug =
            surfaces.create_surface(base_resolution, desc_pickbuffer_debug.clone())?;

        Ok(PickBuffers {
            depth,
            depth_proxy: Mutex::new(
                SurfaceProxy::new(
                    gpu,
                    surfaces.get(depth),
                    Some(dxgi::Format::R32FloatX8x24Typeless),
                    false,
                )
                .context("Failed to create depth surface proxy")?,
            ),
            pickbuffer,
            pickbuffer_proxy: Mutex::new(
                SurfaceProxy::new(
                    gpu,
                    surfaces.get(pickbuffer),
                    Some(dxgi::Format::R32Uint),
                    false,
                )
                .context("Failed to create pickbuffer surface proxy")?,
            ),
            pickbuffer_debug,
            pickbuffer_sample,
            selection_request: AtomicCell::new(None),
            selection_ready: AtomicBool::new(false),
            selected_entity: None,
        })
    }

    pub fn start(&self, renderer: &Renderer, cmd: &mut CommandList, surfaces: &Surfaces) {
        self.clear(cmd, surfaces);
        self.bind(cmd, renderer);

        cmd.state = PipelineState::new(Some(0), Some(2), Some(2), Some(0));
        cmd.state_override = PipelineState::new(Some(0), Some(2), Some(2), Some(0));
        // Set scissor rect?
    }

    pub fn end(&self, renderer: &Renderer, cmd: &mut CommandList, surfaces: &Surfaces) {
        self.pickbuffer_proxy
            .lock()
            .update(cmd, surfaces.get(self.pickbuffer));
        if renderer.sample_entity(cmd, self) {
            self.pickbuffer_sample.copy_to_staging(cmd);
            self.selection_ready.store(true, Ordering::Relaxed);
        }
        // Drop scissor rect?
    }

    pub fn bind(&self, cmd: &mut CommandList, renderer: &Renderer) {
        renderer.bind_surfaces(cmd, &[self.pickbuffer], Some(self.depth));
    }

    pub fn bind_depth_only(&self, cmd: &mut CommandList, renderer: &Renderer) {
        renderer.bind_surfaces(cmd, &[], Some(self.depth));
    }

    pub fn clear(&self, context: &d3d11::DeviceContext, surfaces: &Surfaces) {
        surfaces
            .get(self.pickbuffer)
            .clear_color(context, [0., 0., 0., 0.]);
        surfaces.get(self.depth).clear_depth(context, 0., 0xff);
    }

    pub fn request_selection(&self, x: u32, y: u32) {
        self.selection_request.store(Some((x, y)));
        self.selection_ready.store(false, Ordering::Relaxed);
    }

    pub fn has_request(&self) -> bool {
        self.selection_request.load().is_some()
    }

    pub fn cancel_request(&self) {
        self.selection_request.store(None);
    }

    pub fn finish_request(&self, renderer: &Renderer) -> Option<u32> {
        if !self.selection_ready.fetch_and(false, Ordering::Relaxed) {
            return None;
        }
        Some(
            self.pickbuffer_sample
                .read_element(&renderer.gpu.context(), 0)
                .unwrap(),
        )
    }
}

impl Renderer {
    pub fn submit_view_pickbuffer(self: &Arc<Self>, cmd: &mut CommandList, view: &MainView) {
        self.submit_gbuffer_generation(cmd, view, None);

        view.pickbuffer.start(self, cmd, &view.surfaces);
        {
            cmd.set_override_pixel_shader(self.common.pickbuffer_ps.clone());
            self.pickbuffer_cb.bind(cmd, ShaderStage::Pixel, 7);

            self.submit_stage(
                cmd,
                View::MAIN,
                //RenderStage::DepthPrepass,
                RenderStage::GenerateGbuffer,
                FeatureRendererSubscription::all(),
            );
            cmd.set_override_pixel_shader(None);
        }
        view.pickbuffer.end(self, cmd, &view.surfaces);
    }

    pub fn submit_view_pickbuffer_debug(self: &Arc<Self>, cmd: &mut CommandList, view: &MainView) {
        self.submit_view_pickbuffer(cmd, view);

        {
            profiling::scope!("generate_pickbuffer_debug");
            let _gpuscope = self.profiler.scope(cmd, "generate_pickbuffer_debug");
            cmd.vertex_set_shader(&self.common.blit_vs);
            cmd.pixel_set_shader(&self.common.pickbuffer_debug_ps);

            let surf_pickbuffer = view.surfaces.get(view.pickbuffer.pickbuffer);
            let surf_output = view.surfaces.get(view.output);
            surf_output.bind_single(cmd);
            cmd.pixel_set_shader_resources(0, &[surf_pickbuffer.srv(0)]);

            cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
            cmd.flush_states();
            cmd.set_input_topology(PrimitiveType::TriangleStrip);

            cmd.draw(4, 0);
        }
    }

    fn sample_entity(&self, cmd: &mut CommandList, pickbuffer: &PickBuffers) -> bool {
        let Some(target) = pickbuffer.selection_request.take() else {
            return false;
        };
        cmd_event_span!(cmd, "sample_pickbuffer");
        cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));

        let _gpuscope = self.profiler.scope(cmd, "sample_pickbuffer");
        cmd.compute_set_shader(&self.pickbuffer_sample_cs);

        // Set depth texture as input to t0
        let pb = pickbuffer.pickbuffer_proxy.lock();
        let srv = Some(pb.srv.clone());
        cmd.compute_set_shader_resources(0, &[srv.as_ref()]);

        // Set coordinate to same to to b0
        _ = self.sample_params.write(
            cmd,
            &CoordParams {
                coord: UVec2::new(target.0, target.1),
            },
        );
        self.sample_params.bind(cmd, ShaderStage::Compute, 0);

        // Set up entity output at u0

        cmd.compute_set_unordered_access_views(0, &[Some(&pickbuffer.pickbuffer_sample.uav)], None);

        cmd.dispatch(1, 1, 1);

        cmd.compute_set_unordered_access_views(0, &[None], None);
        cmd.compute_set_shader_resources(0, &[None]);
        true
    }
}
