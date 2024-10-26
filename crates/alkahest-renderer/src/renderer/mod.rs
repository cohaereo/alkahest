mod cubemaps;
pub mod gbuffer;
mod immediate;
use crossbeam::atomic::AtomicCell;
use glam::{Mat4, Quat};
pub use immediate::{ImmediateLabel, LabelAlign};
mod lighting_pass;
mod opaque_pass;
mod pickbuffer;
mod postprocess;
pub mod shader;
mod shadows;
pub use shadows::{ShadowPcfSamples, ShadowQuality};
mod systems;
mod transparents_pass;
mod util;

use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use alkahest_data::{
    occlusion::Aabb,
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
        render::{havok::draw_debugshapes_system, light::ShadowGenerationMode},
        resources::SelectedEntity,
        tags::NodeFilterSet,
        transform::Transform,
        utility::draw_utilities_system,
        visibility::{calculate_view_visibility_system, ViewVisibility, VisibilityHelper},
        Scene,
    },
    gpu::SharedGpuContext,
    gpu_event, gpu_profile_event,
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
    Color,
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

    pub settings: RendererSettings,

    pub ssao: SsaoRenderer,
    matcap: MatcapRenderer,
    pub immediate: ImmediateRenderer,
    cubemap_renderer: CubemapRenderer,
    pub pickbuffer: Pickbuffer,

    pub time: AtomicCell<Time>,
    last_frame: Instant,
    pub delta_time: f64,
    pub frame_index: AtomicUsize,

    pub active_view: usize,
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
            settings: RendererSettings::default(),
            time: AtomicCell::new(Time::now()),
            last_frame: Instant::now(),
            delta_time: 0.0,
            frame_index: AtomicUsize::default(),
            active_shadow_generation_mode: ShadowGenerationMode::StationaryOnly,
            lastfilters: NodeFilterSet::default(),
            active_view: 0,
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

        let frustum = view.frustum();
        scene.run_system_once_with(frustum, calculate_view_visibility_system);

        self.update_shadow_maps(scene);

        {
            gpu_profile_event!(self.gpu, "view_0");
            self.bind_view(view, 0);

            self.draw_atmosphere(scene);
            // if self.render_settings.depth_prepass {
            //     self.draw_depth_prepass(scene);
            // }
            self.draw_opaque_pass(scene);
            self.draw_lighting_pass(scene);
            self.draw_shading_pass(scene);
            self.draw_transparents_pass(scene);

            self.draw_postprocessing_pass(scene);

            if self.pickbuffer.selection_request.load().is_some() {
                self.draw_pickbuffer(scene, resources.get::<SelectedEntity>().selected());
            }
        }

        if self.settings.debug_view.is_gamma_converter() {
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

            gpu_profile_event!(self.gpu, "final_or_debug_view");
            let pipeline = self
                .render_globals
                .pipelines
                .get_debug_view_pipeline(self.settings.debug_view);

            self.gpu
                .current_states
                .store(StateSelection::new(Some(0), Some(0), Some(0), Some(0)));
            self.execute_global_pipeline(pipeline, "final_or_debug_view");
        }

        if !self.settings.debug_view.is_gamma_converter() {
            self.draw_view_overlay(scene, resources);
        }

        self.gpu.blit_texture(
            &self.data.lock().gbuffers.shading_result.view,
            self.gpu.swapchain_target.read().as_ref().unwrap(),
            // final_combine and final_combine_no_film_curve already apply gamma correction
            !matches!(
                self.settings.debug_view,
                RenderDebugView::None | RenderDebugView::NoFilmCurve
            ),
        );

        {
            let data = self.data.lock();
            data.gbuffers
                .depth
                .copy_to_staging(&data.gbuffers.depth_staging);
        }

        self.frame_index.fetch_add(1, Ordering::Relaxed);
    }

    fn draw_view_overlay(&self, scene: &mut Scene, resources: &AppResources) {
        gpu_profile_event!(self.gpu, "view_overlay");

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
        // scene.run_system_once_with(resources.get::<RendererShared>().clone(), draw_aabb_system);

        if let Some(selected) = resources.get::<SelectedEntity>().selected() {
            if scene
                .get_entity(selected)
                .map_or(true, |v| v.get::<ViewVisibility>().is_visible(0))
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

            if let Some(bounds) = scene.get::<Aabb>(selected) {
                let transform =
                    if let Some(t) = scene.get::<Transform>(selected) {
                        t.local_to_world()
                    } else {
                        Mat4::IDENTITY
                    } * Transform::new(bounds.center(), Quat::IDENTITY, bounds.extents())
                        .local_to_world();

                self.immediate.cube_outline(
                    transform,
                    resources
                        .get::<SelectedEntity>()
                        .select_fade_color(Color::from_rgb(0.5, 0.26, 0.06), None),
                );
            }
        }

        self.gpu.restore_state(&dxstate);
    }

    fn bind_view(&self, view: &impl View, index: usize) {
        *self.active_view.pocus() = index;
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
                game_time: self.time.load().elapsed(),
                render_time: self.time.load().elapsed(),
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
        self.pocus().settings = settings;
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
            TfxRenderStage::Transparents => self.settings.stage_transparent,
            TfxRenderStage::Decals => self.settings.stage_decals,
            TfxRenderStage::DecalsAdditive => self.settings.stage_decals_additive,
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
            TfxFeatureRenderer::StaticObjects => self.settings.feature_statics.contains(flags_to_check),
            TfxFeatureRenderer::TerrainPatch => self.settings.feature_terrain.contains(flags_to_check),
            TfxFeatureRenderer::RigidObject | TfxFeatureRenderer::DynamicObjects => self.settings.feature_dynamics.contains(flags_to_check),
            TfxFeatureRenderer::SkyTransparent => self.settings.feature_sky.contains(flags_to_check),
            TfxFeatureRenderer::Water => self.settings.feature_water.contains(flags_to_check),
            TfxFeatureRenderer::SpeedtreeTrees => self.settings.feature_decorators.contains(flags_to_check),
            TfxFeatureRenderer::Cubemaps => self.settings.feature_cubemaps,
            _ => true,
        });

        stages_ok && features_ok
    }
}

// Workarounds until we (eventually) get default literals: https://github.com/serde-rs/serde/issues/368
fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RendererSettings {
    pub vsync: bool,
    pub ssao: bool,
    #[serde(skip)]
    pub matcap: bool,
    pub shadow_quality: ShadowQuality,
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
    pub feature_fxaa: bool,

    #[serde(skip, default = "default_true")]
    pub stage_transparent: bool,
    #[serde(skip, default = "default_true")]
    pub stage_decals: bool,
    #[serde(skip, default = "default_true")]
    pub stage_decals_additive: bool,

    #[serde(skip, default = "default_false")]
    pub fxaa_noise: bool,

    // #[serde(skip, default = "default_true")]
    // pub depth_prepass: bool,
    #[serde(skip)]
    pub debug_view: RenderDebugView,
}

impl Default for RendererSettings {
    fn default() -> Self {
        Self {
            vsync: true,
            ssao: true,
            matcap: false,
            shadow_quality: ShadowQuality::Medium,
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
            feature_fxaa: true,

            stage_transparent: true,
            stage_decals: true,
            stage_decals_additive: true,

            fxaa_noise: false,

            // depth_prepass: true,
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
    SpecularOnly,

    Emissive,
    EmissiveIntensity,
    EmissiveLuminance,

    GreyDiffuse,

    Depth,
    DepthEdges,
    DepthGradient,
    DepthWalkable,

    ValidLayeredMetalness,
    ValidSmoothnessHeatmap,
    ValidSourceColorBrightness,
    ValidSourceColorSaturation,
}

impl RenderDebugView {
    /// Does this view convert gamma/color space?
    pub fn is_gamma_converter(&self) -> bool {
        matches!(self, Self::None | Self::NoFilmCurve)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Time {
    Instant(Instant),
    Fixed(f32),
}

impl Time {
    pub fn now() -> Self {
        Self::Instant(Instant::now())
    }

    pub fn fixed(fixed: f32) -> Self {
        Self::Fixed(fixed)
    }

    pub fn elapsed(&self) -> f32 {
        match self {
            Self::Instant(time) => time.elapsed().as_secs_f32(),
            Self::Fixed(time) => *time,
        }
    }

    pub fn to_fixed(&self) -> Self {
        Self::Fixed(self.elapsed())
    }

    pub fn to_instant(&self) -> Self {
        match self {
            Self::Instant(time) => Self::Instant(*time),
            Self::Fixed(time) => Self::Instant(Instant::now() - Duration::from_secs_f32(*time)),
        }
    }
}
