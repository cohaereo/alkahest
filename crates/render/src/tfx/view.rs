use std::sync::Arc;

use alkahest_data::tfx::FeatureRendererSubscription;
use d3d11::dxgi;
use glam::{Mat4, Vec3};
use parking_lot::Mutex;

use crate::{
    renderer::{
        submit::buffers::{Gbuffers, LightBuffers, WaterBuffers},
        surface::{SizeRelativity, SurfaceDesc, SurfaceHandle, SurfaceProxy, Surfaces},
    },
    Gpu,
};

pub struct View {
    pub(crate) position: Vec3,
    pub(crate) world_to_camera: Mat4,
    pub(crate) camera_to_projective: Mat4,

    pub(crate) surfaces: Arc<Surfaces>,
    pub(crate) resolution: (u32, u32),
    pub(crate) gbuffers: Gbuffers,
    pub(crate) lighting: LightBuffers,
    pub(crate) water: WaterBuffers,

    pub(crate) shading_result: SurfaceHandle,
    pub(crate) shading_result_read: Mutex<SurfaceProxy>,
    pub output: SurfaceHandle,

    pub subscribed_features: FeatureRendererSubscription,

    pub settings: RenderSettings,
}

impl View {
    pub fn new(gpu: &Gpu, resolution: (u32, u32)) -> anyhow::Result<Self> {
        let surfaces = Arc::new(Surfaces::new(gpu.device.clone(), resolution));
        let gbuffers = Gbuffers::create(gpu, &surfaces, resolution)?;
        let lighting = LightBuffers::create(&surfaces, resolution)?;
        let water = WaterBuffers::create(&surfaces, resolution)?;

        let shading_result = surfaces.create_surface(
            resolution,
            SurfaceDesc::builder("shading_result", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R11g11b10Float)
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

        Ok(Self {
            position: Vec3::ZERO,
            world_to_camera: Mat4::IDENTITY,
            camera_to_projective: Mat4::IDENTITY,
            resolution,
            surfaces,
            gbuffers,
            lighting,
            water,
            shading_result,
            shading_result_read,
            output,
            subscribed_features: FeatureRendererSubscription::all(),
            settings: RenderSettings::default(),
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

    pub fn resolution(&self) -> (u32, u32) {
        self.resolution
    }

    pub fn surfaces(&self) -> &Arc<Surfaces> {
        &self.surfaces
    }
}

pub struct RenderSettings {
    pub exposure_scale: f32,
    pub vertex_ao: bool,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            exposure_scale: 0.050,
            vertex_ao: true,
        }
    }
}
