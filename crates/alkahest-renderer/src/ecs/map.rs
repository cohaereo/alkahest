use std::collections::HashMap;

use alkahest_data::map::{SMapAtmosphere, SStaticAmbientOcclusion};
use anyhow::Context;
use bevy_ecs::{prelude::Component, system::Resource};
use destiny_pkg::TagHash;
use glam::Vec3;

use crate::{
    gpu::{texture::Texture, GpuContext},
    handle::Handle,
    loaders::{
        texture::load_texture,
        vertex_buffer::{load_vertex_buffer, VertexBuffer},
    },
    tfx::externs::{self, TextureView},
};

#[derive(Resource)]
pub struct MapAtmosphere {
    _data: SMapAtmosphere,
    lookup_0: Option<Texture>,
    _lookup_1: Option<Texture>,
    lookup_2: Option<Texture>,
    _lookup_3: Option<Texture>,
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
            _data: data,
            lookup_0,
            _lookup_1: lookup_1,
            lookup_2,
            _lookup_3: lookup_3,
            unkd0,
        })
    }

    pub fn update_extern(&self, x: &mut externs::Atmosphere, far: bool) {
        let lf_unk30: TextureView = self
            .lookup_0
            .as_ref()
            .map(|l| l.view.clone().into())
            .unwrap_or_default();
        // let lf_unk40: TextureView = self
        //     .lookup_1
        //     .as_ref()
        //     .map(|l| l.view.clone().into())
        //     .unwrap_or_default();
        let lf_unk48: TextureView = self
            .lookup_2
            .as_ref()
            .map(|l| l.view.clone().into())
            .unwrap_or_default();
        // let lf_unk58: TextureView = self
        //     .lookup_3
        //     .as_ref()
        //     .map(|l| l.view.clone().into())
        //     .unwrap_or_default();
        let unkd0: TextureView = self
            .unkd0
            .as_ref()
            .map(|l| l.view.clone().into())
            .unwrap_or_default();

        // if x.unk58.is_null() {
        //     x.unk58 = x.unk40.clone();
        // }

        if far {
            x.unk40 = lf_unk30.clone();
        } else {
            x.unk40 = lf_unk48.clone();
        }
        x.unk58 = lf_unk48.clone();
        x.unk80 = unkd0.clone();
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

#[derive(Resource)]
pub struct MapStaticAO {
    pub ao_buffer: VertexBuffer,
    pub offset_map: HashMap<u64, u32>,
}

impl MapStaticAO {
    pub fn from_tag(gpu: &GpuContext, tag: &SStaticAmbientOcclusion) -> anyhow::Result<Self> {
        let ao_buffer =
            load_vertex_buffer(gpu, tag.ao0.buffer).context("Failed to load AO vertex buffer")?;
        let offset_map = tag
            .ao0
            .mappings
            .iter()
            .map(|m| (m.identifier, m.offset))
            .collect();

        Ok(MapStaticAO {
            ao_buffer,
            offset_map,
        })
    }
}
