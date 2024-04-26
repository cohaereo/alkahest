use std::mem::transmute;

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
pub enum TfxFeatureType {
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

impl TigerReadable for TfxFeatureType {
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
