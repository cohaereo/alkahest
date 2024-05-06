pub mod gbuffer;
mod immediate;
mod lighting_pass;
mod opaque_pass;
mod shader;
mod systems;
mod transparents_pass;

use std::{
    cell::UnsafeCell,
    sync::{atomic::Ordering, Arc},
    thread::Scope,
    time::Instant,
};

use alkahest_data::tfx::TfxShaderStage;
use crossbeam::epoch::Atomic;
use glam::Vec3;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};

use crate::{
    ecs::Scene,
    gpu::SharedGpuContext,
    gpu_event,
    handle::Handle,
    hocus,
    loaders::AssetManager,
    postprocess::ssao::SsaoRenderer,
    renderer::{gbuffer::GBuffer, immediate::ImmediateRenderer},
    shader::matcap::MatcapRenderer,
    tfx::{
        externs,
        externs::{ExternStorage, Frame},
        globals::RenderGlobals,
        scope::ScopeFrame,
        technique::Technique,
        view::View,
    },
    util::color::Color,
};

pub type RendererShared = Arc<Renderer>;

pub struct Renderer {
    pub gpu: SharedGpuContext,

    pub render_globals: RenderGlobals,
    pub data: Mutex<RendererData>,

    pub render_settings: RendererSettings,
    pub ssao: SsaoRenderer,
    matcap: MatcapRenderer,
    pub immediate: ImmediateRenderer,

    pub time: Instant,
    last_frame: Instant,
    pub delta_time: f64,
}

pub struct RendererData {
    pub asset_manager: AssetManager,
    pub gbuffers: GBuffer,
    pub externs: ExternStorage,
}

impl Renderer {
    pub fn create(gpu: SharedGpuContext, window_size: (u32, u32)) -> anyhow::Result<Arc<Self>> {
        let render_globals =
            RenderGlobals::load(gpu.clone()).expect("Failed to load render globals");

        Ok(Arc::new(Self {
            data: Mutex::new(RendererData {
                asset_manager: AssetManager::new(gpu.clone()),
                gbuffers: GBuffer::create(window_size, gpu.clone())?,
                externs: ExternStorage::default(),
            }),
            ssao: SsaoRenderer::new(gpu.clone())?,
            matcap: MatcapRenderer::new(gpu.clone())?,
            immediate: ImmediateRenderer::new(gpu.clone())?,
            gpu,
            render_globals,
            render_settings: RendererSettings::default(),
            time: Instant::now(),
            last_frame: Instant::now(),
            delta_time: 0.0,
        }))
    }

    pub fn get_technique_shared(&self, handle: &Handle<Technique>) -> Option<Arc<Technique>> {
        let data = self.data.lock();
        data.asset_manager.techniques.get_shared(handle)
    }

    pub fn render_world(&self, view: &impl View, scene: &Scene) {
        gpu_event!(self.gpu, "view_0");
        self.begin_world_frame(scene);

        self.data.lock().externs.view = Some({
            let mut e = externs::View::default();
            view.update_extern(&mut e);
            e
        });

        self.render_globals
            .scopes
            .view
            .bind(self)
            .expect("Failed to bind view scope");

        self.draw_atmosphere(scene);
        self.draw_opaque_pass(scene);
        self.draw_lighting_pass(scene);
        self.draw_shading_pass(scene);
        self.draw_transparents_pass(scene);

        self.gpu.blit_texture(
            &self.data.lock().gbuffers.shading_result.view,
            &self.gpu.swapchain_target.read().as_ref().unwrap(),
        );
    }

    fn begin_world_frame(&self, _scene: &Scene) {
        hocus!(self).delta_time = self.last_frame.elapsed().as_secs_f64();
        hocus!(self).last_frame = Instant::now();

        {
            let externs = &mut self.data.lock().externs;
            externs.frame = Frame {
                game_time: self.time.elapsed().as_secs_f32(),
                render_time: self.time.elapsed().as_secs_f32(),
                delta_game_time: self.delta_time as f32,
                specular_lobe_3d_lookup: self
                    .render_globals
                    .textures
                    .specular_lobe_3d_lookup
                    .view
                    .clone()
                    .into(),
                specular_lobe_lookup: self
                    .render_globals
                    .textures
                    .specular_lobe_lookup
                    .view
                    .clone()
                    .into(),
                specular_tint_lookup: self
                    .render_globals
                    .textures
                    .specular_tint_lookup
                    .view
                    .clone()
                    .into(),
                iridescence_lookup: self
                    .render_globals
                    .textures
                    .iridescence_lookup
                    .view
                    .clone()
                    .into(),

                ..externs.frame.clone()
            };

            if let Some(frame_cb) = self
                .render_globals
                .scopes
                .frame
                .stage_pixel
                .as_ref()
                .unwrap()
                .cbuffer
                .as_ref()
            {
                assert!(
                    std::mem::size_of_val(frame_cb.data_array())
                        >= std::mem::size_of::<ScopeFrame>()
                );

                let scope_data = ScopeFrame::from(&externs.frame);
                unsafe {
                    (frame_cb.data_array().as_ptr() as *mut ScopeFrame).write(scope_data);
                    let slot = self
                        .render_globals
                        .scopes
                        .frame
                        .stage_pixel
                        .as_ref()
                        .unwrap()
                        .stage
                        .constant_buffer_slot as u32;

                    frame_cb.bind(slot, TfxShaderStage::Pixel);
                    frame_cb.bind(slot, TfxShaderStage::Vertex);
                    frame_cb.bind(slot, TfxShaderStage::Compute);
                }
            } else {
                panic!("Frame scope does not have a pixel stage cbuffer!!");
            }
        }
    }

    pub fn set_render_settings(&self, settings: RendererSettings) {
        hocus!(self).render_settings = settings;
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RendererSettings {
    pub vsync: bool,
    pub ssao: bool,
    pub atmosphere: bool,
    pub matcap: bool,
}

impl Default for RendererSettings {
    fn default() -> Self {
        Self {
            vsync: true,
            ssao: true,
            atmosphere: false,
            matcap: false,
        }
    }
}
