use std::fmt::{Display, Formatter};

use bitflags::bitflags;
use int_enum::IntEnum;
use tiger_parse::{tiger_type, TigerReadable};

use super::features::dynamic::RenderStageSubscription;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, IntEnum, Hash, Eq)]
pub enum RenderStage {
    GenerateGbuffer = 0,
    Decals,
    InvestmentDecals,
    ShadowGenerate,
    LightingApply,
    LightProbeApply,
    DecalsAdditive,
    Transparents,
    Distortion,
    LightShaftOcclusion,
    SkinPrepass,
    LensFlares,
    DepthPrepass,
    WaterReflection,
    PostprocessTransparentStencil,
    Impulse,
    Reticle,
    WaterRipples,
    MaskSunLight,
    Volumetrics,
    Cubemaps,
    PostprocessScreen,
    WorldForces,
    ComputeSkinning,
}

impl RenderStage {
    pub const COUNT: usize = Self::ComputeSkinning as usize + 1;

    pub fn to_flag(&self) -> RenderStageSubscription {
        RenderStageSubscription::from_bits(1 << *self as u32).unwrap()
    }
}

impl Display for RenderStage {
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

        write!(f, "{n}")
    }
}

impl TigerReadable for RenderStage {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        let v = u8::read_ds_endian(reader, endian)?;
        Self::try_from(v).map_err(|_| tiger_parse::Error::EnumVariantOutOfRange(v as usize))
    }

    const SIZE: usize = 1;
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, IntEnum)]
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

impl Display for TfxFeatureRenderer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let n = match self {
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
        };

        write!(f, "{n}")
    }
}

impl TigerReadable for TfxFeatureRenderer {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        let v = u8::read_ds_endian(reader, endian)?;
        Self::try_from(v).map_err(|_| tiger_parse::Error::EnumVariantOutOfRange(v as usize))
    }

    const SIZE: usize = 1;
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct FeatureRendererSubscription : u32 {
        const STATIC_OBJECTS            = 1 << TfxFeatureRenderer::StaticObjects as u32;
        const DYNAMIC_OBJECTS           = 1 << TfxFeatureRenderer::DynamicObjects as u32;
        const EXAMPLE_ENTITY            = 1 << TfxFeatureRenderer::ExampleEntity as u32;
        const SKINNED_OBJECT            = 1 << TfxFeatureRenderer::SkinnedObject as u32;
        const GEAR                      = 1 << TfxFeatureRenderer::Gear as u32;
        const RIGID_OBJECT              = 1 << TfxFeatureRenderer::RigidObject as u32;
        const CLOTH                     = 1 << TfxFeatureRenderer::Cloth as u32;
        const CHUNKED_INSTANCE_OBJECTS  = 1 << TfxFeatureRenderer::ChunkedInstanceObjects as u32;
        const SOFT_DEFORMABLE           = 1 << TfxFeatureRenderer::SoftDeformable as u32;
        const TERRAIN_PATCH             = 1 << TfxFeatureRenderer::TerrainPatch as u32;
        const SPEEDTREE_TREES           = 1 << TfxFeatureRenderer::SpeedtreeTrees as u32;
        const EDITOR_TERRAIN_TILE       = 1 << TfxFeatureRenderer::EditorTerrainTile as u32;
        const EDITOR_MESH               = 1 << TfxFeatureRenderer::EditorMesh as u32;
        const BATCHED_EDITOR_MESH       = 1 << TfxFeatureRenderer::BatchedEditorMesh as u32;
        const EDITOR_DECAL              = 1 << TfxFeatureRenderer::EditorDecal as u32;
        const PARTICLES                 = 1 << TfxFeatureRenderer::Particles as u32;
        const CHUNKED_LIGHTS            = 1 << TfxFeatureRenderer::ChunkedLights as u32;
        const DEFERRED_LIGHTS           = 1 << TfxFeatureRenderer::DeferredLights as u32;
        const SKY_TRANSPARENT           = 1 << TfxFeatureRenderer::SkyTransparent as u32;
        const WIDGET                    = 1 << TfxFeatureRenderer::Widget as u32;
        const DECALS                    = 1 << TfxFeatureRenderer::Decals as u32;
        const DYNAMIC_DECALS            = 1 << TfxFeatureRenderer::DynamicDecals as u32;
        const ROAD_DECALS               = 1 << TfxFeatureRenderer::RoadDecals as u32;
        const WATER                     = 1 << TfxFeatureRenderer::Water as u32;
        const LENS_FLARES               = 1 << TfxFeatureRenderer::LensFlares as u32;
        const VOLUMETRICS               = 1 << TfxFeatureRenderer::Volumetrics as u32;
        const CUBEMAPS                  = 1 << TfxFeatureRenderer::Cubemaps as u32;
    }
}

impl FeatureRendererSubscription {
    pub const fn all_but(feature: TfxFeatureRenderer) -> Self {
        Self::all().difference(Self::from_bits_truncate(1 << feature as u32))
    }

    pub fn is_subscribed(&self, feature: TfxFeatureRenderer) -> bool {
        self.contains(Self::from_bits_truncate(1 << feature as u32))
    }
}

// TODO(cohae): Duplicate struct, used in TFX bytecode in the renderer
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, IntEnum)]
pub enum ShaderStage {
    Pixel = 1,
    Vertex = 2,
    Geometry = 3,
    Hull = 4,
    Compute = 5,
    Domain = 6,
}

impl ShaderStage {
    pub fn from_index(index: u8) -> Option<Self> {
        match index {
            1 => Some(ShaderStage::Pixel),
            2 => Some(ShaderStage::Vertex),
            3 => Some(ShaderStage::Geometry),
            4 => Some(ShaderStage::Hull),
            5 => Some(ShaderStage::Compute),
            6 => Some(ShaderStage::Domain),
            _ => None,
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            ShaderStage::Pixel => "PS",
            ShaderStage::Vertex => "VS",
            ShaderStage::Geometry => "GS",
            ShaderStage::Hull => "HS",
            ShaderStage::Compute => "CS",
            ShaderStage::Domain => "DS",
        }
    }
}

impl TigerReadable for ShaderStage {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        let v = u8::read_ds_endian(reader, endian)?;
        Self::try_from(v).map_err(|_| tiger_parse::Error::EnumVariantOutOfRange(v as usize))
    }

    const SIZE: usize = 1;
}

#[derive(Debug, PartialEq, Copy, Clone, IntEnum)]
#[repr(u8)]
pub enum PrimitiveType {
    PointList = 0,
    LineList = 1,
    LineStrip = 2,
    Triangles = 3,
    TriangleStrip = 5,
}

impl TigerReadable for PrimitiveType {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        let v = u8::read_ds_endian(reader, endian)?;
        Self::try_from(v).map_err(|_| tiger_parse::Error::EnumVariantOutOfRange(v as usize))
    }

    const SIZE: usize = 1;
}

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[tiger_type]
#[repr(u8)]
pub enum LodCategory {
    /// main geometry lod0
    Lod_0_0 = 0,
    /// grip/stock lod0
    Lod_0_1 = 1,
    /// stickers lod0
    Lod_0_2 = 2,
    /// internal geom lod0
    Lod_0_3 = 3,
    /// low poly geom lod1
    Lod_1_0 = 4,
    /// low poly geom lod2
    Lod_2_0 = 7,
    /// grip/stock/scope lod2
    Lod_2_1 = 8,
    /// low poly geom lod3
    Lod_3_0 = 9,
    /// detail lod0
    Lod_Detail = 10,
}

impl LodCategory {
    pub fn is_highest_detail(&self) -> bool {
        matches!(
            self,
            Self::Lod_0_0 | Self::Lod_0_1 | Self::Lod_0_2 | Self::Lod_0_3 | Self::Lod_Detail
        )
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, IntEnum)]
pub enum ExternIndex {
    None,
    Frame,
    View,
    Deferred,
    DeferredLight,
    DeferredUberLight,
    DeferredShadow,
    Atmosphere,
    RigidModel,
    EditorMesh,
    EditorMeshMaterial,
    EditorDecal,
    EditorTerrain,
    EditorTerrainPatch,
    EditorTerrainDebug,
    SimpleGeometry,
    UiFont,
    CuiView,
    CuiObject,
    CuiBitmap,
    CuiVideo,
    CuiStandard,
    CuiHud,
    CuiScreenspaceBoxes,
    CuiDrawingShader,
    TextureVisualizer,
    Generic,
    Particle,
    ParticleDebug,
    GearDyeVisualizationMode,
    ScreenArea,
    Mlaa,
    Msaa,
    Hdao,
    DownsampleTextureGeneric,
    DownsampleDepth,
    Ssao,
    VolumetricObscurance,
    Postprocess,
    TextureSet,
    Transparent,
    Vignette,
    GlobalLighting,
    ShadowMask,
    ObjectEffect,
    Decal,
    DecalSetTransform,
    DynamicDecal,
    DecoratorWind,
    TextureCameraLighting,
    VolumeFog,
    Fxaa,
    Smaa,
    Cmaa,
    Letterbox,
    DepthOfField,
    PostprocessInitialDownsample,
    CopyDepth,
    DisplacementMotionBlur,
    DebugShader,
    MinmaxDepth,
    SdsmBiasAndScale,
    SdsmBiasAndScaleTextures,
    ComputeShadowMapData,
    ComputeLocalLightShadowMapData,
    BilateralUpsample,
    HealthOverlay,
    LightProbeDominantLight,
    LightProbeLightInstance,
    Water,
    LensFlare,
    ScreenShader,
    Scaler,
    GammaControl,
    SpeedtreePlacements,
    Reticle,
    Distortion,
    WaterDebug,
    ScreenAreaInput,
    WaterDepthPrepass,
    OverheadVisibilityMapMain,
    OverheadVisibilityMapInterior,
    OverheadVisibilityMapOcclusion,
    ParticleCompute,
    CubemapFiltering,
    CubemapDiffuseCapture,
    ParticleFastpath,
    VolumetricsPass,
    TemporalReprojection,
    FxaaCompute,
    UberDepth,
    GearDye,
    Cubemaps,
    ShadowBlendWithPrevious,
    DebugShadingOutput,
    Ssao3D,
    WaterDisplacement,
    PatternBlending,
    UiHdrTransform,
    PlayerCenteredCascadedGrid,
    SoftDeform,
    RaymarchedAtmosphereVolume,
    Gtao,
    Taa,
    FirstPersonShadows,
    SkinningCompute,
    Ssr,
    StylizedDropShadow,
    SsrTrace,
    TextureCameraDownsample,
    WaterReflection,
    OcclusionCullingReprojectDepth,
    OcclusionCullingVisibilityTesting,
    VariableRateShading,
}

impl ExternIndex {
    pub const COUNT: usize = Self::VariableRateShading as usize + 1;
}
