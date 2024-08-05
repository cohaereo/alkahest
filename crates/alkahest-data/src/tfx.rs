use std::{
    fmt::{Display, Formatter},
    mem::transmute,
};

use tiger_parse::TigerReadable;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TfxRenderStage {
    GenerateGbuffer = 0,
    Decals = 1,
    InvestmentDecals = 2,
    ShadowGenerate = 3,
    LightingApply = 4,
    LightProbeApply = 5,
    DecalsAdditive = 6,
    Transparents = 7,
    Distortion = 8,
    LightShaftOcclusion = 9,
    SkinPrepass = 10,
    LensFlares = 11,
    DepthPrepass = 12,
    WaterReflection = 13,
    PostprocessTransparentStencil = 14,
    Impulse = 15,
    Reticle = 16,
    WaterRipples = 17,
    MaskSunLight = 18,
    Volumetrics = 19,
    Cubemaps = 20,
    PostprocessScreen = 21,
    WorldForces = 22,
    ComputeSkinning = 23,
}

impl TfxRenderStage {
    pub const VARIANTS: [TfxRenderStage; 24] = [
        Self::GenerateGbuffer,
        Self::Decals,
        Self::InvestmentDecals,
        Self::ShadowGenerate,
        Self::LightingApply,
        Self::LightProbeApply,
        Self::DecalsAdditive,
        Self::Transparents,
        Self::Distortion,
        Self::LightShaftOcclusion,
        Self::SkinPrepass,
        Self::LensFlares,
        Self::DepthPrepass,
        Self::WaterReflection,
        Self::PostprocessTransparentStencil,
        Self::Impulse,
        Self::Reticle,
        Self::WaterRipples,
        Self::MaskSunLight,
        Self::Volumetrics,
        Self::Cubemaps,
        Self::PostprocessScreen,
        Self::WorldForces,
        Self::ComputeSkinning,
    ];
}

impl Display for TfxRenderStage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let n = match self {
            Self::GenerateGbuffer => "generate_gbuffer",
            Self::Decals => "decals",
            Self::InvestmentDecals => "investment_decals",
            Self::ShadowGenerate => "shadow_generate",
            Self::LightingApply => "lighting_apply",
            Self::LightProbeApply => "light_probe_apply",
            Self::DecalsAdditive => "decals_additive",
            Self::Transparents => "transparents",
            Self::Distortion => "distortion",
            Self::LightShaftOcclusion => "light_shaft_occlusion",
            Self::SkinPrepass => "skin_prepass",
            Self::LensFlares => "lens_flares",
            Self::DepthPrepass => "depth_prepass",
            Self::WaterReflection => "water_reflection",
            Self::PostprocessTransparentStencil => "postprocess_transparent_stencil",
            Self::Impulse => "impulse",
            Self::Reticle => "reticle",
            Self::WaterRipples => "water_ripples",
            Self::MaskSunLight => "mask_sun_light",
            Self::Volumetrics => "volumetrics",
            Self::Cubemaps => "cubemaps",
            Self::PostprocessScreen => "postprocess_screen",
            Self::WorldForces => "world_forces",
            Self::ComputeSkinning => "compute_skinning",
        };

        write!(f, "{}", n)
    }
}

impl TigerReadable for TfxRenderStage {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        Ok(unsafe { transmute(u8::read_ds_endian(reader, endian)?) })
    }

    const ZEROCOPY: bool = true;
    const SIZE: usize = 1;
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub enum TfxFeatureRenderer {
    StaticObjects = 0,
    DynamicObjects = 1,
    ExampleEntity = 2,
    SkinnedObject = 3,
    Gear = 4,
    RigidObject = 5,
    Cloth = 6,
    ChunkedInstanceObjects = 7,
    SoftDeformable = 8,
    TerrainPatch = 9,
    SpeedtreeTrees = 10,
    EditorTerrainTile = 11,
    EditorMesh = 12,
    BatchedEditorMesh = 13,
    EditorDecal = 14,
    Particles = 15,
    ChunkedLights = 16,
    DeferredLights = 17,
    SkyTransparent = 18,
    Widget = 19,
    Decals = 20,
    DynamicDecals = 21,
    RoadDecals = 22,
    Water = 23,
    LensFlares = 24,
    Volumetrics = 25,
    Cubemaps = 26,
}

impl TfxFeatureRenderer {
    pub fn short(&self) -> &'static str {
        match self {
            TfxFeatureRenderer::StaticObjects => "static_object",
            TfxFeatureRenderer::DynamicObjects => "dynamic_object",
            TfxFeatureRenderer::ExampleEntity => "example_entity",
            TfxFeatureRenderer::SkinnedObject => "skinned_object",
            TfxFeatureRenderer::Gear => "gear",
            TfxFeatureRenderer::RigidObject => "rigid_object",
            TfxFeatureRenderer::Cloth => "cloth",
            TfxFeatureRenderer::ChunkedInstanceObjects => "chunked_instance_object",
            TfxFeatureRenderer::SoftDeformable => "soft_deformable",
            TfxFeatureRenderer::TerrainPatch => "terrain_patch",
            TfxFeatureRenderer::SpeedtreeTrees => "speedtree_trees",
            TfxFeatureRenderer::EditorTerrainTile => "editor_terrain_tile",
            TfxFeatureRenderer::EditorMesh => "editor_mesh",
            TfxFeatureRenderer::BatchedEditorMesh => "batched_editor_mesh",
            TfxFeatureRenderer::EditorDecal => "editor_decal",
            TfxFeatureRenderer::Particles => "particles",
            TfxFeatureRenderer::ChunkedLights => "chunked_light",
            TfxFeatureRenderer::DeferredLights => "deferred_light",
            TfxFeatureRenderer::SkyTransparent => "sky_transparent",
            TfxFeatureRenderer::Widget => "widget",
            TfxFeatureRenderer::Decals => "decal",
            TfxFeatureRenderer::DynamicDecals => "dynamic_decal",
            TfxFeatureRenderer::RoadDecals => "road_decal",
            TfxFeatureRenderer::Water => "water",
            TfxFeatureRenderer::LensFlares => "lens_flare",
            TfxFeatureRenderer::Volumetrics => "volumetrics",
            TfxFeatureRenderer::Cubemaps => "cubemap",
        }
    }
}

impl TigerReadable for TfxFeatureRenderer {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        Ok(unsafe { transmute(u8::read_ds_endian(reader, endian)?) })
    }

    const ZEROCOPY: bool = true;
    const SIZE: usize = 1;
}

// TODO(cohae): Duplicate struct, used in TFX bytecode in the renderer
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum TfxShaderStage {
    Pixel = 1,
    Vertex = 2,
    Geometry = 3,
    Hull = 4,
    Compute = 5,
    Domain = 6,
}

impl TfxShaderStage {
    pub fn from_tfx_value(value: u8) -> Result<TfxShaderStage, String> {
        match value >> 5 {
            1 => Ok(TfxShaderStage::Pixel),
            2 => Ok(TfxShaderStage::Vertex),
            3 => Ok(TfxShaderStage::Geometry),
            4 => Ok(TfxShaderStage::Hull),
            5 => Ok(TfxShaderStage::Compute),
            6 => Ok(TfxShaderStage::Domain),
            _ => Err(format!("Invalid TFX shader stage: {}", value)),
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            TfxShaderStage::Pixel => "PS",
            TfxShaderStage::Vertex => "VS",
            TfxShaderStage::Geometry => "GS",
            TfxShaderStage::Hull => "HS",
            TfxShaderStage::Compute => "CS",
            TfxShaderStage::Domain => "DS",
        }
    }
}

impl TigerReadable for TfxShaderStage {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        Ok(unsafe { transmute(u8::read_ds_endian(reader, endian)?) })
    }

    const ZEROCOPY: bool = true;
    const SIZE: usize = 1;
}
