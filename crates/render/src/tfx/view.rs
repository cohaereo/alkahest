use std::sync::Arc;

use alkahest_data::tfx::FeatureRendererSubscription;
use d3d11::dxgi;
use glam::{Mat4, Vec3, Vec4};
use parking_lot::{Mutex, RwLock};

use crate::{
    Gpu, Renderer,
    renderer::{
        autoexposure::AutoExposureSystem,
        submit::buffers::{AtmosphereBuffers, BloomBuffers, Gbuffers, LightBuffers, WaterBuffers},
        surface::{
            SizeRelativity, SurfaceDesc, SurfaceHandle, SurfaceProxy, SurfaceScale, Surfaces,
        },
    },
};

pub struct View {
    pub(crate) position: Vec3,
    pub(crate) world_to_camera: Mat4,
    pub(crate) camera_to_projective: Mat4,

    pub surfaces: Arc<Surfaces>,
    pub(crate) resolution: (u32, u32),

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

    pub subscribed_features: FeatureRendererSubscription,

    pub settings: RenderSettings,

    pub autoexposure: AutoExposureSystem,
    frame_index: u64,
}

impl View {
    pub fn new(gpu: &Gpu, resolution: (u32, u32)) -> anyhow::Result<Self> {
        let surfaces = Arc::new(Surfaces::new(gpu.device.clone(), resolution));
        let gbuffers = Gbuffers::create(gpu, &surfaces, resolution)?;
        let lighting = LightBuffers::create(&surfaces, resolution)?;
        let water = WaterBuffers::create(&surfaces, resolution)?;
        let bloom = BloomBuffers::create(gpu, &surfaces, resolution)?;
        let atmosphere = AtmosphereBuffers::create(&surfaces, resolution)?;

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
            position: Vec3::ZERO,
            world_to_camera: Mat4::IDENTITY,
            camera_to_projective: Mat4::IDENTITY,
            resolution,
            surfaces,
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
            subscribed_features: FeatureRendererSubscription::all(),
            settings: RenderSettings::default(),
            autoexposure: AutoExposureSystem::default(),
            frame_index: 0,
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
    }

    pub fn update_autoexposure(&mut self, gpu: &Gpu, delta_time: f32) {
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

    pub fn resolution(&self) -> (u32, u32) {
        self.resolution
    }

    pub fn surfaces(&self) -> &Arc<Surfaces> {
        &self.surfaces
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
        }
    }
}
