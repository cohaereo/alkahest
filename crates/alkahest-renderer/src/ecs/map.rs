use alkahest_data::map::SMapAtmosphere;
use bevy_ecs::{prelude::Component, system::Resource};
use destiny_pkg::TagHash;
use glam::Vec3;

use crate::{
    gpu::{texture::Texture, GpuContext},
    handle::Handle,
    loaders::texture::load_texture,
    tfx::externs::{self, TextureView},
};

// TODO(cohae): This should probably be a resource, since there can only be one per map
#[derive(Resource)]
pub struct MapAtmosphere {
    data: SMapAtmosphere,
    lookup_0: Option<Texture>,
    lookup_1: Option<Texture>,
    lookup_2: Option<Texture>,
    lookup_3: Option<Texture>,
    unkd0: Option<Texture>,
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
        let unkd0 = data
            .unkd0
            .hash32_checked()
            .map(|hash| load_texture(gctx, hash))
            .transpose()?;

        Ok(MapAtmosphere {
            data,
            lookup_0,
            lookup_1,
            lookup_2,
            lookup_3,
            unkd0,
        })
    }

    pub fn update_extern(&self, x: &mut externs::Atmosphere) {
        let lf_unk30: TextureView = self
            .lookup_0
            .as_ref()
            .map(|l| l.view.clone().into())
            .unwrap_or_default();
        let lf_unk40: TextureView = self
            .lookup_1
            .as_ref()
            .map(|l| l.view.clone().into())
            .unwrap_or_default();
        let lf_unk48: TextureView = self
            .lookup_2
            .as_ref()
            .map(|l| l.view.clone().into())
            .unwrap_or_default();
        let lf_unk58: TextureView = self
            .lookup_3
            .as_ref()
            .map(|l| l.view.clone().into())
            .unwrap_or_default();
        let unkd0: TextureView = self
            .unkd0
            .as_ref()
            .map(|l| l.view.clone().into())
            .unwrap_or_default();

        // if x.unk58.is_null() {
        //     x.unk58 = x.unk40.clone();
        // }

        x.unk40 = lf_unk30.clone();
        x.unk58 = lf_unk48.clone();
        x.light_shaft_optical_depth = unkd0.clone();
    }
}

#[derive(Component)]
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

#[derive(Component, Clone)]
pub struct NodeMetadata {
    pub entity_tag: TagHash,
    pub world_id: u64,
    pub source_table: TagHash,
    pub source_table_resource_offset: u64,
    pub resource_type: u32,

    pub name: Option<String>,
}
