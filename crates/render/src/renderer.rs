pub mod autoexposure;
pub mod globals;
pub mod submit;
pub mod surface;
pub mod util;
pub mod visibility;

use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use alkahest_core::ConVars;
use alkahest_data::tfx::{
    ExternIndex, FeatureRendererSubscription, features::ao::SStaticAmbientOcclusion,
    texture::DxgiFormat,
};
use anyhow::Context;
use crossbeam::atomic::AtomicCell;
use d3d11::dxgi;
use glam::{Mat4, Vec4};
use globals::RenderGlobals;
use parking_lot::{Mutex, RwLock, RwLockReadGuard};
use surface::Surfaces;

use crate::{
    Gpu,
    asset::{
        AssetManager, Handle,
        texture::{Texture, TextureHandle},
        vertex_buffer::VertexBuffer,
    },
    feature::immediate::ImmediateShapeRenderer,
    gpu::{cbuffer::ConstantBuffer, debug_text::DebugTextRenderer, profiler::D3D11Profiler},
    object::{RenderObject, RenderObjectHandle},
    renderer::submit::bloom::PostProcessScope,
    tfx::{
        externs::Externs,
        packet::FramePacket,
        scope::{CascadeScope, TempFrameScope},
        view::RenderSettings,
    },
    util::{
        arena::Arena,
        threading::{CommandListPool, ThreadMutCell},
    },
};

const DEBUG_SHADER: &str = include_str!("../builtin/shaders/debug.hlsl");
const CLEAR_AO_SHADER: &str = include_str!("../builtin/shaders/clear_ao.hlsl");
const SHADOW_MAP_SHADER: &str = include_str!("../builtin/shaders/shadow_map.hlsl");
const BLIT_SHADER: &str = include_str!("../builtin/shaders/blit_srgb.hlsl");
const BLIT_FAKE_WEAPON_SHADER: &str = include_str!("../builtin/shaders/blit_fake_weapon.hlsl");

pub struct Renderer {
    pub gpu: Arc<Gpu>,
    pub asset_manager: AssetManager,
    // pub timestamps: TimestampManager,
    pub immediate: Mutex<ImmediateShapeRenderer>,
    pub debug_text: Mutex<DebugTextRenderer>,
    pub externs: ThreadMutCell<Externs>,

    pub objects: RwLock<Arena<RenderObject>>,
    pub frame_packet: RwLock<FramePacket>,
    pub globals: RenderGlobals,

    surfaces: RwLock<Arc<Surfaces>>,
    pub cmd_pool: Arc<CommandListPool>,

    frame_scope: ConstantBuffer<TempFrameScope>,
    transparent_advanced_scope: ConstantBuffer<[Vec4; 8]>,
    debug_vs: d3d11::VertexShader,
    debug_ps: d3d11::PixelShader,
    clear_ao_vs: d3d11::VertexShader,
    clear_ao_ps: d3d11::PixelShader,
    clear_ao_all_ps: d3d11::PixelShader,
    shadow_map_vs: d3d11::VertexShader,
    shadow_map_ps: d3d11::PixelShader,
    cascade_scope: ConstantBuffer<CascadeScope>,

    pub ao: RwLock<Option<SStaticAmbientOcclusion>>,
    pub ao_buffer: RwLock<Option<Handle<VertexBuffer>>>,

    pub common: CommonResources,
    active_feature_renderers: AtomicCell<FeatureRendererSubscription>,
    placeholder_textures:
        RwLock<HashMap<(ExternIndex, u32), (Texture, d3d11::UnorderedAccessView)>>,

    debug_cbuffer: ConstantBuffer<Mat4>,
    postprocess_cbuffer: ConstantBuffer<PostProcessScope>,
    pub profiler: D3D11Profiler,

    pub settings: RwLock<RenderSettings>,
}

unsafe impl Send for Renderer {}
unsafe impl Sync for Renderer {}

static RENDERER_GLOBAL: OnceLock<Arc<Renderer>> = OnceLock::new();
impl Renderer {
    pub fn new(gpu: Arc<Gpu>) -> anyhow::Result<Self> {
        ConVars::register("render.sky", true);
        ConVars::register("render.global_lighting", false);
        ConVars::register("render.threaded_submit", true);
        ConVars::register("render.patch_light_shader", false);
        ConVars::register("render.vertex_ao_workaround", true);
        ConVars::register("render.vao_buffer", false);
        ConVars::register("render.ssao", true);
        ConVars::register("render.max_distance", 350.0f32);

        ConVars::register("render.feature.static_objects", true);
        ConVars::register("render.feature.terrain_patches", true);
        ConVars::register("render.feature.rigid_objects", true);
        ConVars::register("render.feature.chunked_lights", true);
        ConVars::register("render.feature.deferred_lights", true);
        ConVars::register("render.feature.sky_transparent", true);
        ConVars::register("render.feature.decals", true);
        ConVars::register("render.feature.dynamic_decals", true);
        ConVars::register("render.feature.road_decals", true);
        ConVars::register("render.feature.water", true);
        ConVars::register("render.feature.volumetrics", true);
        ConVars::register("render.feature.cubemaps", true);

        // Placeholder surface container
        // TODO(cohae): Can we remove this at some point?
        // The only reason we need this is because we store surface handles in externs, so the interpreter needs to be able to access their SRVs through the renderer
        let surfaces = Arc::new(Surfaces::new(gpu.device.clone(), (128, 128)));

        let (debug_vs, debug_ps) =
            gpu.compile_shader_vs_ps("debug", DEBUG_SHADER, "mainVS", "mainPS")?;

        let (clear_ao_vs, clear_ao_ps) =
            gpu.compile_shader_vs_ps("clear_ao", CLEAR_AO_SHADER, "mainVS", "mainPS")?;

        let (_clear_ao_vs, clear_ao_all_ps) =
            gpu.compile_shader_vs_ps("clear_ao", CLEAR_AO_SHADER, "mainVS", "mainPSall")?;

        let (shadow_map_vs, shadow_map_ps) =
            gpu.compile_shader_vs_ps("shadow_map", SHADOW_MAP_SHADER, "mainVS", "mainPS")?;

        let globals = RenderGlobals::load(&gpu).context("Failed to load render globals")?;
        Ok(Self {
            externs: ThreadMutCell::new(Externs::new(&globals)),
            globals,
            // timestamps: TimestampManager::new(&gpu.device)?,
            asset_manager: AssetManager::new(&gpu),
            debug_text: Mutex::new(DebugTextRenderer::create(&gpu)?),
            immediate: Mutex::new(
                ImmediateShapeRenderer::new(&gpu).context("Failed to create immediate renderer")?,
            ),
            objects: RwLock::new(Arena::new()),
            frame_packet: RwLock::new(FramePacket::default()),
            ao: RwLock::new(None),
            ao_buffer: RwLock::new(None),
            surfaces: RwLock::new(surfaces),
            cmd_pool: CommandListPool::new(&gpu).into(),
            frame_scope: ConstantBuffer::create(&gpu, None)?,
            transparent_advanced_scope: ConstantBuffer::create(&gpu, None)?,
            debug_cbuffer: ConstantBuffer::create(&gpu, Some(&Mat4::ZERO))?,
            postprocess_cbuffer: ConstantBuffer::create(&gpu, None)?,

            debug_vs,
            debug_ps,
            clear_ao_vs,
            clear_ao_ps,
            clear_ao_all_ps,
            shadow_map_vs,
            shadow_map_ps,
            cascade_scope: ConstantBuffer::create(&gpu, None)?,

            common: CommonResources::load(&gpu)?,

            profiler: D3D11Profiler::new(&gpu),
            gpu,
            active_feature_renderers: AtomicCell::new(FeatureRendererSubscription::all()),
            placeholder_textures: RwLock::new(HashMap::new()),

            settings: RwLock::new(RenderSettings::default()),
        })
    }

    pub fn set_instance(renderer: Arc<Self>) {
        if RENDERER_GLOBAL.set(renderer).is_err() {
            panic!("GPU is already initialized!");
        }
    }

    pub fn is_initialized() -> bool {
        RENDERER_GLOBAL.get().is_some()
    }

    pub fn instance() -> &'static Arc<Renderer> {
        RENDERER_GLOBAL.get().expect("GPU is not yet initialized!")
    }

    pub fn add_object(&self, object: RenderObject) -> RenderObjectHandle {
        RenderObjectHandle(self.objects.write().insert(object))
    }

    /// Returns a read-only reference to the currently bound view's surfaces
    pub fn surfaces(&self) -> RwLockReadGuard<'_, Arc<Surfaces>> {
        self.surfaces.read()
    }

    pub fn clone_object(&self, handle: RenderObjectHandle) -> Option<RenderObjectHandle> {
        let objects = self.objects.read();
        let object = objects.get(handle.into()).and_then(|o| o.dyn_clone())?;
        drop(objects);

        Some(RenderObjectHandle(self.objects.write().insert(object)))
    }

    pub fn remove_object(&self, handle: RenderObjectHandle) {
        let mut objects = self.objects.write();
        objects.remove(handle.into());
    }

    pub fn is_object_loaded(&self, handle: RenderObjectHandle) -> bool {
        let objects = self.objects.read();
        if let Some(object) = objects.get(handle.into()) {
            object.is_loaded()
        } else {
            false
        }
    }

    pub fn resize_swapchain(&self, resolution: (u32, u32)) {
        self.gpu.resize_swapchain(resolution);
    }

    pub fn get_extern_placeholder_texture<F>(&self, index: ExternIndex, offset: usize, f: F)
    where
        F: FnOnce(&Texture, &d3d11::UnorderedAccessView),
    {
        let mut placeholder_textures = self.placeholder_textures.write();

        let (texture, uav) = placeholder_textures
            .entry((index, offset as u32))
            .or_insert_with(|| {
                let gpu = &Renderer::instance().gpu;
                let data = match (index, offset) {
                    (ExternIndex::Atmosphere, _) => bytemuck::cast_slice(&[[0u8, 0, 0, 0]; 4]),
                    (ExternIndex::ShadowMask, _) => {
                        bytemuck::cast_slice(&[[0xffu8, 0xff, 0xff, 0xff]; 4])
                    }
                    (ExternIndex::Frame, 0xC0) => {
                        bytemuck::cast_slice(&[[0xffu8, 0xff, 0xff, 0xff]; 4])
                    }

                    (ExternIndex::Transparent, 0x00) // Atmosphere (near?)
                    | (ExternIndex::Transparent, 0x08) // Atmosphere (3x2)
                    | (ExternIndex::Transparent, 0x10) // Atmosphere (far?)
                    | (ExternIndex::Transparent, 0x50) // t21, unused?
                    => {
                        bytemuck::cast_slice(&[[0u8, 0, 0, 0]; 4])
                    }
                    (ExternIndex::Transparent, 0x48) // t20
                    => {
                        bytemuck::cast_slice(&[[0u8, 0, 0, 0xff]; 4])
                    }
                    (ExternIndex::Water, 0x28) => bytemuck::cast_slice(&[[127u8, 127, 0, 0]; 4]), // RG16_UNORM
                    // VolumetricsPass inputs are generally the results of the last pass, alpha is *ALWAYS* cleared to zero
                    (ExternIndex::VolumetricsPass, _) => bytemuck::cast_slice(&[[0u8, 0, 0, 0]; 4]),
                    _ => bytemuck::cast_slice(&[[0u8, 0, 0, 255]; 4]),
                };
                assert_eq!(data.len(), (2 * 2) * 4);

                let texture = Texture::load_2d_raw(
                    gpu,
                    2,
                    2,
                    data,
                    DxgiFormat::from(dxgi::Format::R8g8b8a8Unorm),
                    Some(&format!("unknown extern {index:?}+0x{offset:X}")),
                    true,
                )
                .unwrap();

                let TextureHandle::Texture2D(tex) = &texture.handle else {
                    unreachable!()
                };
                let uav = gpu
                    .create_unordered_access_view(tex, None)
                    .expect("Failed to create uav for placeholder texture");

                (texture, uav)
            });

        f(texture, uav);
    }

    pub fn settings(&self) -> RwLockReadGuard<'_, RenderSettings> {
        self.settings.read()
    }
}

impl Renderer {
    pub fn begin_frame(&self) {
        // self.timestamps.begin_frame();
        self.asset_manager.remove_unreferenced();
    }

    pub fn present_frame(&self, vsync: bool) {
        self.gpu.present(vsync);
        // self.timestamps.collect();
        self.debug_text.lock().clear();
    }
}

pub struct CommonResources {
    default_lut: Texture,

    pub shadowmap_vs_t2: Texture,
    pub vertex_color_fallback: VertexBuffer,

    blit_vs: d3d11::VertexShader,
    blit_ps: d3d11::PixelShader,
    blit_ps_linear: d3d11::PixelShader,

    blit_fw_vs: d3d11::VertexShader,
    blit_fw_ps: d3d11::PixelShader,

    temporary_sky_hemisphere: Texture,
    temporary_vignette: Texture,
    temporary_health_overlay: Texture,
    temporary_bloom: Texture,

    temporary_atmos: Texture,
    temporary_depth_angle_lookup: Texture,
    temporary_depth_lookup: Texture,

    pub disable_skinning_vs: d3d11::VertexShader,

    sampler_point: d3d11::SamplerState,
    sampler_linear: d3d11::SamplerState,
}

impl CommonResources {
    pub fn load(gpu: &Gpu) -> anyhow::Result<Self> {
        let mut default_lut_data = vec![];
        for z in 0..32 {
            for y in 0..32 {
                for x in 0..32 {
                    let r = x as f32 / 31.0;
                    let g = y as f32 / 31.0;
                    let b = z as f32 / 31.0;
                    default_lut_data.push((r * 255.0) as u8);
                    default_lut_data.push((g * 255.0) as u8);
                    default_lut_data.push((b * 255.0) as u8);
                    default_lut_data.push(255);
                }
            }
        }

        let default_lut = Texture::load_3d_raw(
            gpu,
            32,
            32,
            32,
            &default_lut_data,
            dxgi::Format::R8g8b8a8Unorm,
            Some("lut3d_temp"),
        )?;

        let (blit_vs, blit_ps) =
            gpu.compile_shader_vs_ps("blit", BLIT_SHADER, "mainVS", "mainPS")?;
        let (_, blit_ps_linear) =
            gpu.compile_shader_vs_ps("blit", BLIT_SHADER, "mainVS", "mainPS_linear")?;

        let (blit_fw_vs, blit_fw_ps) =
            gpu.compile_shader_vs_ps("blit_fw", BLIT_FAKE_WEAPON_SHADER, "mainVS", "mainPS")?;

        let shadowmap_vs_t2 = Texture::load_2d_raw(
            &gpu,
            1,
            1,
            &[0, 0, 255, 255],
            dxgi::Format::R8g8b8a8UnormSrgb,
            Some("shadowmap_vs_t2"),
            false,
        )?;

        let vertex_color_fallback =
            VertexBuffer::load_data(gpu, bytemuck::cast_slice(&[[255u8, 255, 255, 255]]), 4)?;

        let sampler_point = gpu.create_sampler_state(&d3d11::SamplerDesc {
            filter: d3d11::Filter::MinMagMipPoint,
            address_u: d3d11::TextureAddress::Clamp,
            address_v: d3d11::TextureAddress::Clamp,
            address_w: d3d11::TextureAddress::Clamp,
            ..Default::default()
        })?;

        let sampler_linear = gpu.create_sampler_state(&d3d11::SamplerDesc {
            filter: d3d11::Filter::MinMagMipLinear,
            address_u: d3d11::TextureAddress::Clamp,
            address_v: d3d11::TextureAddress::Clamp,
            address_w: d3d11::TextureAddress::Clamp,
            ..Default::default()
        })?;

        Ok(Self {
            temporary_sky_hemisphere: Texture::load_2d_dds(
                gpu,
                include_bytes!("../builtin/textures/sky_hemisphere_cosmo.dds"),
            )?,
            shadowmap_vs_t2,
            vertex_color_fallback,
            default_lut,
            temporary_vignette: Texture::load_2d_dds(
                gpu,
                include_bytes!("../builtin/textures/vignette.dds"),
            )?,
            temporary_health_overlay: Texture::load_2d_dds(
                gpu,
                include_bytes!("../builtin/textures/health_overlay.dds"),
            )?,
            temporary_bloom: Texture::load_2d_dds(
                gpu,
                include_bytes!("../builtin/textures/screen_area_0x18.dds"),
            )?,
            temporary_atmos: Texture::load_2d_dds(
                gpu,
                include_bytes!("../builtin/textures/atmos0.dds"),
            )?,
            temporary_depth_angle_lookup: Texture::load_2d_dds(
                gpu,
                include_bytes!("../builtin/textures/depth_angle_lookup.dds"),
            )?,
            temporary_depth_lookup: Texture::load_2d_dds(
                gpu,
                include_bytes!("../builtin/textures/depth_lookup.dds"),
            )?,
            disable_skinning_vs: gpu.create_vertex_shader(include_bytes!(
                "../builtin/shaders/skinning-noskinning.vs.cso"
            ))?,
            blit_vs,
            blit_ps,
            blit_ps_linear,
            blit_fw_vs,
            blit_fw_ps,
            sampler_point,
            sampler_linear,
        })
    }
}
