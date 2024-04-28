use parking_lot::Mutex;

use crate::{
    gpu::SharedGpuContext,
    loaders::AssetManager,
    tfx::{externs::ExternStorage, gbuffer::GBuffer},
};

pub struct Renderer {
    pub gpu: SharedGpuContext,
    pub data: Mutex<RendererData>,
}

pub struct RendererData {
    pub asset_manager: AssetManager,
    pub gbuffers: GBuffer,
    pub externs: ExternStorage,
}

impl Renderer {
    pub fn new(gpu: SharedGpuContext) -> anyhow::Result<Self> {
        Ok(Self {
            data: Mutex::new(RendererData {
                asset_manager: AssetManager::new(gpu.clone()),
                gbuffers: GBuffer::create((4, 4), gpu.clone())?,
                externs: ExternStorage::default(),
            }),
            gpu,
        })
    }
}
