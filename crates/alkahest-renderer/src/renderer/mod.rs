mod cubemaps;
pub mod gbuffer;
mod immediate;
pub use immediate::LabelAlign;
mod lighting_pass;
mod opaque_pass;
mod pickbuffer;
pub mod shader;
mod shadows;
mod systems;
mod transparents_pass;
mod util;

use std::{ops::Deref, sync::Arc, time::Instant};

use alkahest_data::{
    technique::StateSelection,
    tfx::{TfxFeatureRenderer, TfxRenderStage, TfxShaderStage},
};
use anyhow::Context;
use bevy_ecs::system::{Resource, RunSystemOnce};
use bitflags::bitflags;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use strum::{EnumCount, EnumIter};
use windows::Win32::Graphics::Direct3D11::D3D11_VIEWPORT;

use crate::{
    ecs::{
        common::Hidden,
        render::{havok::draw_debugshapes_system, light::ShadowGenerationMode},
        resources::SelectedEntity,
        tags::NodeFilterSet,
        utility::draw_utilities_system,
        Scene,
    },
    gpu::SharedGpuContext,
    gpu_event,
    handle::Handle,
    loaders::AssetManager,
    postprocess::ssao::SsaoRenderer,
    renderer::{
        cubemaps::CubemapRenderer, gbuffer::GBuffer, immediate::ImmediateRenderer,
        pickbuffer::Pickbuffer,
    },
    resources::AppResources,
    shader::matcap::MatcapRenderer,
    tfx::{
        externs::{self, ExternStorage, Frame},
        globals::RenderGlobals,
        scope::ScopeFrame,
        technique::Technique,
        view::View,
    },
    util::Hocus,
};

#[derive(Resource)]
pub struct RendererShared(Arc<Renderer>);

impl Clone for RendererShared {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Deref for RendererShared {
    type Target = Arc<Renderer>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Renderer {
    pub gpu: SharedGpuContext,

    pub render_globals: RenderGlobals,
    pub data: Mutex<RendererData>,

    pub render_settings: RendererSettings,

    pub ssao: SsaoRenderer,
    matcap: MatcapRenderer,
    pub immediate: ImmediateRenderer,
    cubemap_renderer: CubemapRenderer,
    pub pickbuffer: Pickbuffer,

    pub time: Instant,
    last_frame: Instant,
    pub delta_time: f64,
    pub frame_index: usize,

    // Hacky way to obtain these filters for now
    pub lastfilters: NodeFilterSet,
    pub active_shadow_generation_mode: ShadowGenerationMode,
}

pub struct RendererData {
    pub asset_manager: AssetManager,
    pub gbuffers: GBuffer,
    pub externs: ExternStorage,
}

impl Renderer {
    pub fn create(
        gpu: SharedGpuContext,
        window_size: (u32, u32),
        disable_asset_loading: bool,
    ) -> anyhow::Result<RendererShared> {
        let render_globals =
            RenderGlobals::load(gpu.clone()).expect("Failed to load render globals");

        Ok(RendererShared(Arc::new(Self {
            data: Mutex::new(RendererData {
                asset_manager: if disable_asset_loading {
                    AssetManager::new_disabled(gpu.clone())
                } else {
                    AssetManager::new(gpu.clone())
                },
                gbuffers: GBuffer::create(window_size, gpu.clone())?,
                externs: ExternStorage::default(),
            }),
            ssao: SsaoRenderer::new(gpu.clone()).context("failed to create SsaoRenderer")?,
            matcap: MatcapRenderer::new(gpu.clone()).context("failed to create MatcapRenderer")?,
            immediate: ImmediateRenderer::new(gpu.clone())
                .context("failed to create ImmediateRenderer")?,
            cubemap_renderer: CubemapRenderer::new(gpu.clone())
                .context("failed to create CubemapRenderer")?,
            pickbuffer: Pickbuffer::new(gpu.clone(), window_size)
                .context("failed to create Pickbuffer")?,
            gpu,
            render_globals,
            render_settings: RendererSettings::default(),
            time: Instant::now(),
            last_frame: Instant::now(),
            delta_time: 0.0,
            frame_index: 0,
            active_shadow_generation_mode: ShadowGenerationMode::StationaryOnly,
            lastfilters: NodeFilterSet::default(),
        })))
    }

    pub fn get_technique_shared(&self, handle: &Handle<Technique>) -> Option<Arc<Technique>> {
        let data = self.data.lock();
        data.asset_manager.techniques.get_shared(handle)
    }

    pub fn render_world(&self, view: &impl View, scene: &mut Scene, resources: &AppResources) {
        self.pocus().lastfilters = resources.get::<NodeFilterSet>().clone();

        // Make sure immediate labels have been drained completely
        let _ = self.immediate.drain_labels();

        self.begin_world_frame(scene);

        self.update_shadow_maps(scene);

        {
            gpu_event!(self.gpu, "view_0");
            self.bind_view(view);

            self.draw_atmosphere(scene);
            self.draw_opaque_pass(scene);
            self.draw_lighting_pass(scene);
            self.draw_shading_pass(scene);
            self.draw_transparents_pass(scene);

            if self.pickbuffer.selection_request.load().is_some() {
                self.draw_pickbuffer(scene, resources.get::<SelectedEntity>().selected());
            }
        }

        if self.render_settings.debug_view.is_gamma_converter() {
            self.draw_view_overlay(scene, resources);
        }

        unsafe {
            {
                let mut data = self.data.lock();
                data.gbuffers
                    .shading_result
                    .copy_to(&data.gbuffers.shading_result_read);
                data.externs.postprocess = Some(externs::Postprocess {
                    unk00: data.gbuffers.shading_result_read.view.clone().into(),
                    ..Default::default()
                });

                self.gpu.context().OMSetRenderTargets(
                    Some(&[Some(
                        // self.gpu.swapchain_target.read().as_ref().unwrap().clone(),
                        data.gbuffers.shading_result.render_target.clone(),
                    )]),
                    None,
                );
            }

            gpu_event!(self.gpu, "final_or_debug_view");
            let pipeline = self
                .render_globals
                .pipelines
                .get_debug_view_pipeline(self.render_settings.debug_view);

            self.gpu
                .current_states
                .store(StateSelection::new(Some(0), Some(0), Some(0), Some(0)));
            self.execute_global_pipeline(pipeline, "final_or_debug_view");
        }

        if !self.render_settings.debug_view.is_gamma_converter() {
            self.draw_view_overlay(scene, resources);
        }

        self.gpu.blit_texture(
            &self.data.lock().gbuffers.shading_result.view,
            self.gpu.swapchain_target.read().as_ref().unwrap(),
            // final_combine and final_combine_no_film_curve already apply gamma correction
            !matches!(
                self.render_settings.debug_view,
                RenderDebugView::None | RenderDebugView::NoFilmCurve
            ),
        );

        {
            let data = self.data.lock();
            data.gbuffers
                .depth
                .copy_to_staging(&data.gbuffers.depth_staging);
        }

        self.pocus().frame_index = self.frame_index.wrapping_add(1);
    }

    fn draw_view_overlay(&self, scene: &mut Scene, resources: &AppResources) {
        gpu_event!(self.gpu, "view_overlay");

        self.gpu
            .current_states
            .store(StateSelection::new(Some(0), Some(2), Some(2), Some(1)));
        self.gpu.flush_states();

        let dxstate = self.gpu.backup_state();
        unsafe {
            self.gpu.context().OMSetRenderTargets(
                Some(&dxstate.render_targets),
                &self.data.lock().gbuffers.depth.view,
            );
        }

        // TODO(cohae): Move debug shapes to a separate system
        scene.run_system_once_with(
            resources.get::<RendererShared>().clone(),
            draw_debugshapes_system,
        );
        scene.run_system_once_with(
            resources.get::<RendererShared>().clone(),
            draw_utilities_system,
        );

        if let Some(selected) = resources.get::<SelectedEntity>().selected() {
            if !scene
                .get_entity(selected)
                .map_or(true, |v| v.contains::<Hidden>())
            {
                self.draw_outline(
                    scene,
                    selected,
                    resources
                        .get::<SelectedEntity>()
                        .time_selected
                        .elapsed()
                        .as_secs_f32(),
                );
            }
        }

        self.gpu.restore_state(&dxstate);
    }

    fn bind_view(&self, view: &impl View) {
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

        let vp = view.viewport();
        unsafe {
            self.gpu.context().RSSetViewports(Some(&[D3D11_VIEWPORT {
                TopLeftX: vp.origin.x as f32,
                TopLeftY: vp.origin.y as f32,
                Width: vp.size.x as f32,
                Height: vp.size.y as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }]));
        }
    }

    fn begin_world_frame(&self, _scene: &Scene) {
        self.pocus().delta_time = self.last_frame.elapsed().as_secs_f64();
        self.pocus().last_frame = Instant::now();

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
                        .constants
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
        self.pocus().render_settings = settings;
    }

    pub fn resize_buffers(&self, width: u32, height: u32) {
        self.data
            .lock()
            .gbuffers
            .resize((width, height))
            .expect("Failed to resize GBuffer");

        self.pocus()
            .pickbuffer
            .resize((width, height))
            .expect("Failed to resize Pickbuffer");
    }

    /// Checks if we should render the given stage and feature, based on render settings
    #[rustfmt::skip]
    pub fn should_render(&self, stage: Option<TfxRenderStage>, feature: Option<TfxFeatureRenderer>) -> bool {
        let flags_to_check = if self.pickbuffer.is_drawing_selection {
            // An object needs to be visible for it to be selectable
            RenderFeatureVisibility::SELECTABLE | RenderFeatureVisibility::VISIBLE
        } else {
            RenderFeatureVisibility::VISIBLE
        };

        // Can we render based on stages?
        let mut stages_ok = stage.map_or(true, |v| match v {
            TfxRenderStage::Transparents => self.render_settings.stage_transparent,
            TfxRenderStage::Decals => self.render_settings.stage_decals,
            TfxRenderStage::DecalsAdditive => self.render_settings.stage_decals_additive,
            _ => true,
        });

        if stages_ok &&
            stage == Some(TfxRenderStage::ShadowGenerate)
        {
            // If we're drawing terrain patches, we should only generate shadows in stationary only mode
            // StaticObjects may generate stationary as well as moving shadows, so it's not checked here
            if feature == Some(TfxFeatureRenderer::TerrainPatch) && self.active_shadow_generation_mode != ShadowGenerationMode::StationaryOnly {
                stages_ok = false;
            }

            // If we're not drawing statics (static objects/terrain), we should only generate shadows in moving only mode
            if !matches!(feature, Some(TfxFeatureRenderer::TerrainPatch) | Some(TfxFeatureRenderer::StaticObjects)) && self.active_shadow_generation_mode != ShadowGenerationMode::MovingOnly {
                stages_ok = false;
            }
        }

        let features_ok = feature.map_or(true, |v| match v {
            TfxFeatureRenderer::StaticObjects => self.render_settings.feature_statics.contains(flags_to_check),
            TfxFeatureRenderer::TerrainPatch => self.render_settings.feature_terrain.contains(flags_to_check),
            TfxFeatureRenderer::RigidObject | TfxFeatureRenderer::DynamicObjects => self.render_settings.feature_dynamics.contains(flags_to_check),
            TfxFeatureRenderer::SkyTransparent => self.render_settings.feature_sky.contains(flags_to_check),
            TfxFeatureRenderer::Water => self.render_settings.feature_water.contains(flags_to_check),
            TfxFeatureRenderer::SpeedtreeTrees => self.render_settings.feature_decorators.contains(flags_to_check),
            TfxFeatureRenderer::Cubemaps => self.render_settings.feature_cubemaps,
            _ => true,
        });

        stages_ok && features_ok
    }
}

// Workaround until we (eventually) get default literals: https://github.com/serde-rs/serde/issues/368
fn default_true() -> bool {
    true
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RendererSettings {
    pub vsync: bool,
    pub ssao: bool,
    #[serde(skip)]
    pub matcap: bool,
    pub shadows: bool,
    pub shadow_updates_per_frame: usize,

    #[serde(skip, default = "RenderFeatureVisibility::all")]
    pub feature_statics: RenderFeatureVisibility,
    #[serde(skip, default = "RenderFeatureVisibility::all")]
    pub feature_terrain: RenderFeatureVisibility,
    #[serde(skip, default = "RenderFeatureVisibility::all")]
    pub feature_dynamics: RenderFeatureVisibility,
    #[serde(skip, default = "RenderFeatureVisibility::all")]
    pub feature_sky: RenderFeatureVisibility,
    #[serde(skip, default = "RenderFeatureVisibility::all")]
    pub feature_decorators: RenderFeatureVisibility,
    #[serde(skip, default = "RenderFeatureVisibility::all")]
    pub feature_water: RenderFeatureVisibility,
    pub feature_atmosphere: bool,
    pub feature_cubemaps: bool,
    pub feature_global_lighting: bool,

    #[serde(skip, default = "default_true")]
    pub stage_transparent: bool,
    #[serde(skip, default = "default_true")]
    pub stage_decals: bool,
    #[serde(skip, default = "default_true")]
    pub stage_decals_additive: bool,

    #[serde(skip)]
    pub debug_view: RenderDebugView,
}

impl Default for RendererSettings {
    fn default() -> Self {
        Self {
            vsync: true,
            ssao: true,
            matcap: false,
            shadows: true,
            shadow_updates_per_frame: 2,

            feature_statics: RenderFeatureVisibility::all(),
            feature_terrain: RenderFeatureVisibility::all(),
            feature_dynamics: RenderFeatureVisibility::all(),
            feature_sky: RenderFeatureVisibility::all(),
            feature_decorators: RenderFeatureVisibility::all(),
            feature_water: RenderFeatureVisibility::all(),
            feature_atmosphere: false,
            feature_cubemaps: false,
            feature_global_lighting: false,

            stage_transparent: true,
            stage_decals: true,
            stage_decals_additive: true,

            debug_view: RenderDebugView::None,
        }
    }
}

bitflags! {
    #[derive(Serialize, Deserialize, Clone, Copy)]
    pub struct RenderFeatureVisibility : u8 {
        const SELECTABLE = 1 << 0;
        const VISIBLE = 1 << 1;
    }
}

impl Default for RenderFeatureVisibility {
    fn default() -> Self {
        Self::all()
    }
}

#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Serialize,
    Deserialize,
    EnumIter,
    strum::Display,
    EnumCount,
)]
pub enum RenderDebugView {
    #[default]
    None,
    NoFilmCurve,

    GbufferValidation,
    SourceColor,
    Normal,
    NormalEdges,
    Metalness,
    AmbientOcclusion,
    TextureAo,
    Transmission,

    ColoredOvercoatId,
    ColoredOvercoat,

    DiffuseColor,
    DiffuseLight,
    SpecularColor,
    SpecularLight,
    SpecularOcclusion,
    SpecularSmoothness,

    Emissive,
    EmissiveIntensity,
    EmissiveLuminance,

    GreyDiffuse,

    Depth,
    DepthEdges,
    DepthGradient,
    DepthWalkable,
}

impl RenderDebugView {
    /// Does this view convert gamma/color space?
    pub fn is_gamma_converter(&self) -> bool {
        matches!(self, Self::None | Self::NoFilmCurve)
    }
}
