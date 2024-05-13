use alkahest_data::{map::SMapAtmosphere, occlusion::AABB};
use glam::Vec3;

use crate::{
    gpu::{texture::Texture, GpuContext},
    handle::Handle,
    loaders::texture::load_texture,
    tfx::externs,
};

pub struct MapAtmosphere {
    data: SMapAtmosphere,
    lookup_0: Option<Texture>,
    lookup_1: Option<Texture>,
    lookup_2: Option<Texture>,
    lookup_3: Option<Texture>,
}

impl MapAtmosphere {
    pub fn load(gctx: &GpuContext, data: SMapAtmosphere) -> anyhow::Result<MapAtmosphere> {
        let lookup_0 = data
            .lookup_texture_0
            .hash32_checked()
            .map(|hash| load_texture(gctx, hash))
            .transpose()?;
        let lookup_1 = data
            .lookup_texture_1
            .hash32_checked()
            .map(|hash| load_texture(gctx, hash))
            .transpose()?;
        let lookup_2 = data
            .lookup_texture_2
            .hash32_checked()
            .map(|hash| load_texture(gctx, hash))
            .transpose()?;
        let lookup_3 = data
            .lookup_texture_3
            .hash32_checked()
            .map(|hash| load_texture(gctx, hash))
            .transpose()?;

        Ok(MapAtmosphere {
            data,
            lookup_0,
            lookup_1,
            lookup_2,
            lookup_3,
        })
    }

    pub fn update_extern(&self, x: &mut externs::Atmosphere) {
        x.unk30 = self
            .lookup_0
            .as_ref()
            .map(|l| l.view.clone().into())
            .unwrap_or_default();
        x.unk40 = self
            .lookup_1
            .as_ref()
            .map(|l| l.view.clone().into())
            .unwrap_or_default();
        x.unk48 = self
            .lookup_2
            .as_ref()
            .map(|l| l.view.clone().into())
            .unwrap_or_default();
        x.unk58 = self
            .lookup_3
            .as_ref()
            .map(|l| l.view.clone().into())
            .unwrap_or_default();

        x.unk20 = x.unk30.clone();
        x.unk38 = x.unk48.clone();
    }
}

pub struct CubemapVolume {
    pub specular_ibl: Handle<Texture>,
    pub voxel_diffuse: Option<Handle<Texture>>,

    pub extents: Vec3,
    pub name: String,
}

impl CubemapVolume {
    pub fn volume(&self) -> f32 {
        self.extents.x * self.extents.y * self.extents.z
    }
}
