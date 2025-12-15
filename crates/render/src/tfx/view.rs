use std::sync::Arc;

use alkahest_data::tfx::FeatureRendererSubscription;
use d3d11::dxgi;
use glam::{Mat4, Vec3};

use crate::{
    renderer::{
        submit::buffers::{Gbuffers, LightBuffers},
        surface::{SizeRelativity, SurfaceDesc, SurfaceHandle, Surfaces},
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
    pub(crate) shading_result: SurfaceHandle,
    pub output: SurfaceHandle,
    pub subscribed_features: FeatureRendererSubscription,
}

impl View {
    pub fn new(gpu: &Gpu, resolution: (u32, u32)) -> anyhow::Result<Self> {
        let surfaces = Arc::new(Surfaces::new(gpu.device.clone(), resolution));
        let gbuffers = Gbuffers::create(gpu, &surfaces, resolution)?;
        let lighting = LightBuffers::create(&surfaces, resolution)?;

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

        Ok(Self {
            position: Vec3::ZERO,
            world_to_camera: Mat4::IDENTITY,
            camera_to_projective: Mat4::IDENTITY,
            resolution,
            surfaces,
            gbuffers,
            lighting,
            shading_result,
            output,
            subscribed_features: FeatureRendererSubscription::all(),
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
}
