use std::sync::Arc;

use alkahest_data::tfx::{FeatureRendererSubscription, common::AxisAlignedBBox};
use d3d11::dxgi;
use glam::{Mat4, Vec3, Vec4};
use inline_tweak::tweak;
use parking_lot::{Mutex, RwLock};

use crate::{
    Gpu, Renderer,
    renderer::{
        autoexposure::AutoExposureSystem,
        hzb::Hzb,
        submit::buffers::{AtmosphereBuffers, BloomBuffers, Gbuffers, LightBuffers, WaterBuffers},
        surface::{
            SizeRelativity, SurfaceDesc, SurfaceHandle, SurfaceProxy, SurfaceScale, Surfaces,
        },
    },
    visibility::frustum::Frustum,
};

pub enum ViewKind {
    Main(Box<MainView>),
    Shadow(Box<ShadowView>),
}

pub struct View {
    pub name: String,
    pub(crate) position: Vec3,
    pub(crate) world_to_camera: Mat4,
    pub(crate) camera_to_projective: Mat4,
    world_to_projective: Mat4,

    pub culling_frustum: Frustum,
    pub hzb: Hzb,

    pub surfaces: Arc<Surfaces>,
    pub(crate) resolution: (u32, u32),

    pub kind: ViewKind,

    pub subscribed_features: FeatureRendererSubscription,
    pub disable_culling: bool,
}

impl View {
    pub const MAIN: usize = 0;
    pub const SUN: usize = 1;
    pub const FIRST_SHADOW: usize = 2;

    pub fn new_main(
        name: impl Into<String>,
        gpu: &Gpu,
        resolution: (u32, u32),
    ) -> anyhow::Result<Self> {
        Self::new_inner(name, gpu, false, resolution)
    }

    pub fn new_shadow(
        name: impl Into<String>,
        gpu: &Gpu,
        resolution: (u32, u32),
    ) -> anyhow::Result<Self> {
        Self::new_inner(name, gpu, true, resolution)
    }

    fn new_inner(
        name: impl Into<String>,
        gpu: &Gpu,
        is_shadow: bool,
        resolution: (u32, u32),
    ) -> anyhow::Result<Self> {
        let surfaces = Arc::new(Surfaces::new(gpu.device.clone(), resolution));

        let kind = if is_shadow {
            ViewKind::Shadow(Box::new(ShadowView::new(&surfaces, resolution)?))
        } else {
            ViewKind::Main(Box::new(MainView::new(&surfaces, gpu, resolution)?))
        };

        Ok(Self {
            name: name.into(),
            position: Vec3::ZERO,
            world_to_camera: Mat4::IDENTITY,
            camera_to_projective: Mat4::IDENTITY,
            world_to_projective: Mat4::IDENTITY,
            culling_frustum: Frustum::default(),
            hzb: Hzb::EMPTY,
            resolution,
            surfaces,
            kind,
            subscribed_features: FeatureRendererSubscription::all(),
            disable_culling: false,
        })
    }

    pub fn update(
        &mut self,
        world_to_camera: Mat4,
        camera_to_projective: Mat4,
        resolution: (u32, u32),
    ) {
        self.world_to_camera = world_to_camera;
        self.camera_to_projective = camera_to_projective;
        self.position = self.world_to_camera.inverse().transform_point3(Vec3::ZERO);
        self.resolution = resolution;
        self.culling_frustum =
            Frustum::from_view_and_projection(self.world_to_camera, self.camera_to_projective);
    }

    pub fn update_autoexposure(&mut self, gpu: &Gpu, delta_time: f32) {
        if let ViewKind::Main(v) = &mut self.kind {
            v.update_autoexposure(gpu, delta_time)
        }
    }

    pub fn resolution(&self) -> (u32, u32) {
        self.resolution
    }

    pub fn surfaces(&self) -> &Arc<Surfaces> {
        &self.surfaces
    }

    pub fn settings(&self) -> &RenderSettings {
        match &self.kind {
            ViewKind::Main(v) => &v.settings,
            ViewKind::Shadow(v) => &v.settings,
        }
    }

    pub fn settings_mut(&mut self) -> &mut RenderSettings {
        match &mut self.kind {
            ViewKind::Main(v) => &mut v.settings,
            ViewKind::Shadow(v) => &mut v.settings,
        }
    }

    fn is_in_clip_space(&self, aabb: &AxisAlignedBBox) -> bool {
        // Don't bother checking an invalid/uninitialized AABB
        if !aabb.is_valid() {
            return true;
        }

        if !self.culling_frustum.aabb_intersecting(aabb) {
            return false;
        }

        // Project the AABB corners to check how big they appear on screen
        let corners = aabb.points();
        let mut min_ndc = Vec3::splat(f32::MAX);
        let mut max_ndc = Vec3::splat(f32::MIN);
        for corner in &corners {
            let ndc_pos = self.world_to_projective.project_point3(*corner);

            min_ndc = min_ndc.min(ndc_pos);
            max_ndc = max_ndc.max(ndc_pos);
        }

        // If the projected size is too small, consider it not visible
        let ndc_size = max_ndc - min_ndc;
        let screen_size_threshold = tweak!(0.015); // Adjust this threshold as needed
        if ndc_size.x < screen_size_threshold && ndc_size.y < screen_size_threshold {
            return false;
        }

        true
    }

    pub fn is_visible(&self, aabb: &AxisAlignedBBox) -> bool {
        if self.disable_culling {
            return true;
        }

        // Don't bother checking an invalid/uninitialized AABB
        if !aabb.is_valid() {
            return true;
        }

        // if aabb.contains(self.position) {
        //     return true;
        // }

        if !self.is_in_clip_space(aabb) {
            return false;
        }

        self.hzb.is_aabb_visible(aabb)
    }
}

pub struct MainView {
    pub(crate) surfaces: Arc<Surfaces>,
    pub gbuffers: Gbuffers,
    pub(crate) lighting: LightBuffers,
    pub(crate) water: WaterBuffers,
    pub(crate) bloom: BloomBuffers,
    pub(crate) atmosphere: AtmosphereBuffers,
    pub(crate) sun_shadow_map_cascades: Vec<SurfaceHandle>,
    pub(crate) cascade_matrices: RwLock<[Mat4; Renderer::NUM_CASCADES]>,
    pub(crate) shadow_mask: SurfaceHandle,

    pub(crate) shading_result: SurfaceHandle,
    pub(crate) postprocess: SurfaceHandle,
    pub(crate) shading_result_read: Mutex<SurfaceProxy>,
    pub output: SurfaceHandle,
    pub autoexposure: AutoExposureSystem,
    frame_index: u64,
    pub settings: RenderSettings,
}

impl MainView {
    pub fn new(
        surfaces: &Arc<Surfaces>,
        gpu: &Gpu,
        resolution: (u32, u32),
    ) -> anyhow::Result<Self> {
        let gbuffers = Gbuffers::create(gpu, surfaces, resolution)?;
        let lighting = LightBuffers::create(surfaces, resolution)?;
        let water = WaterBuffers::create(surfaces, resolution)?;
        let bloom = BloomBuffers::create(gpu, surfaces, resolution)?;
        let atmosphere = AtmosphereBuffers::create(surfaces, resolution)?;

        let shading_result = surfaces.create_surface(
            resolution,
            SurfaceDesc::builder("shading_result", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R11g11b10Float)
                .build(),
        )?;

        let postprocess = surfaces.create_surface(
            resolution,
            SurfaceDesc::builder("postprocess", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R16g16b16a16Float)
                .build(),
        )?;

        let output = surfaces.create_surface(
            resolution,
            SurfaceDesc::builder("output", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R8g8b8a8UnormSrgb)
                .build(),
        )?;

        let shading_result_read = Mutex::new(
            SurfaceProxy::new(&gpu.device, surfaces.get(shading_result), None, false)
                .expect("Failed to create shading result read proxy"),
        );

        let sun_shadow_map_cascades = (0..Renderer::NUM_CASCADES)
            .map(|i| {
                surfaces.create_surface(
                    (2048, 2048),
                    SurfaceDesc::builder(
                        format!("sun_shadow_cascade_{i}"),
                        SizeRelativity::Absolute,
                    )
                    .format(dxgi::Format::R16Typeless)
                    .view_format(dxgi::Format::R16Unorm)
                    .depth_format(dxgi::Format::D16Unorm)
                    .build(),
                )
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let shadow_mask = surfaces.create_surface(
            resolution,
            SurfaceDesc::builder("shadow_mask", SizeRelativity::RelativeToFramebuffer)
                .scale(SurfaceScale::Half)
                .format(dxgi::Format::R8g8Typeless)
                .view_format(dxgi::Format::R8g8Unorm)
                .build(),
        )?;

        Ok(Self {
            surfaces: surfaces.clone(),
            gbuffers,
            lighting,
            water,
            bloom,
            atmosphere,
            sun_shadow_map_cascades,
            shadow_mask,
            cascade_matrices: RwLock::new([Mat4::IDENTITY; Renderer::NUM_CASCADES]),
            shading_result,
            shading_result_read,
            postprocess,
            output,
            autoexposure: AutoExposureSystem::default(),
            frame_index: 0,
            settings: RenderSettings::default(),
        })
    }

    fn update_autoexposure(&mut self, gpu: &Gpu, delta_time: f32) {
        if self.frame_index > 0 && self.settings.autoexposure {
            let autoexposure_columns = self.bloom.autoexposure_sample_columns_cpu.lock();
            let column_count = autoexposure_columns.res.get_desc().width;

            let mapped_buffer = gpu
                .context()
                .map(&autoexposure_columns.res, 0, d3d11::MapType::Read, false)
                .expect("Failed to map buffer");

            let data = unsafe {
                std::slice::from_raw_parts(mapped_buffer.data as *const Vec4, column_count as usize)
            }
            .to_vec();

            let exposure_result = self.autoexposure.update_from_raw(&data, delta_time);

            self.settings.exposure_scale = exposure_result.exposure_scale;
            // self.settings.exposure_illum_relative = exposure_result.exposure_illum_relative;
        } else {
            self.autoexposure.current_exposure_scale = self.settings.exposure_scale;
            self.autoexposure.current_illum_relative = self.settings.exposure_illum_relative;
        }

        self.frame_index += 1;
    }
}

pub struct ShadowView {
    pub(crate) surfaces: Arc<Surfaces>,
    pub shadow_map: SurfaceHandle,
    pub settings: RenderSettings,
    pub index: usize,
}

impl ShadowView {
    pub const SHADOWMAP_RESOLUTION: u32 = 2048;

    pub fn new(surfaces: &Arc<Surfaces>, resolution: (u32, u32)) -> anyhow::Result<Self> {
        let surface_desc = SurfaceDesc::builder("shadowmap", SizeRelativity::Absolute)
            .format(dxgi::Format::R32Typeless)
            .depth_format(dxgi::Format::D32Float)
            .view_format(dxgi::Format::R32Float)
            .build();

        let shadow_map = surfaces.create_surface(resolution, surface_desc)?;

        Ok(Self {
            surfaces: surfaces.clone(),
            shadow_map,
            settings: RenderSettings::default(),
            index: 0,
        })
    }
}

#[derive(Clone)]
pub struct RenderSettings {
    pub exposure_scale: f32,
    pub exposure_illum_relative: f32,
    pub vertex_ao: bool,
    pub bloom: bool,
    pub volumetrics: bool,
    pub shadows: bool,
    pub autoexposure: bool,
    pub sun_shadows: bool,
    pub anti_aliasing: bool,

    // Performance
    pub multithreading: bool,
    pub instance_culling: bool,
    pub hzb_culling: bool,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            exposure_scale: 0.050,
            exposure_illum_relative: 0.50,
            vertex_ao: true,
            bloom: true,
            volumetrics: true,
            shadows: true,
            autoexposure: true,
            sun_shadows: false,
            anti_aliasing: true,

            multithreading: true,
            instance_culling: true,
            hzb_culling: true,
        }
    }
}
