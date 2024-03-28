use anyhow::Context;
use binrw::binread;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Buffer, ID3D11SamplerState, ID3D11ShaderResourceView,
};

use crate::render::DeviceContextSwapchain;

#[binread]
#[br(repr(u8))]
#[repr(u8)]
#[derive(Clone, Copy, Debug, FromPrimitive, ToPrimitive)]
pub enum TfxExtern {
    None = 0,
    Frame = 1,
    View = 2,
    Deferred = 3,
    DeferredLight = 4,
    DeferredUberLight = 5,
    DeferredShadow = 6,
    Atmosphere = 7,
    RigidModel = 8,
    EditorMesh = 9,
    EditorMeshMaterial = 10,
    EditorDecal = 11,
    EditorTerrain = 12,
    EditorTerrainPatch = 13,
    EditorTerrainDebug = 14,
    SimpleGeometry = 15,
    UiFont = 16,
    CuiView = 17,
    CuiObject = 18,
    CuiBitmap = 19,
    CuiVideo = 20,
    CuiStandard = 21,
    CuiHud = 22,
    CuiScreenspaceBoxes = 23,
    TextureVisualizer = 24,
    Generic = 25,
    Particle = 26,
    ParticleDebug = 27,
    GearDyeVisualizationMode = 28,
    ScreenArea = 29,
    Mlaa = 30,
    Msaa = 31,
    Hdao = 32,
    DownsampleTextureGeneric = 33,
    DownsampleDepth = 34,
    Ssao = 35,
    VolumetricObscurance = 36,
    Postprocess = 37,
    TextureSet = 38,
    Transparent = 39,
    Vignette = 40,
    GlobalLighting = 41,
    ShadowMask = 42,
    ObjectEffect = 43,
    Decal = 44,
    DecalSetTransform = 45,
    DynamicDecal = 46,
    DecoratorWind = 47,
    TextureCameraLighting = 48,
    VolumeFog = 49,
    Fxaa = 50,
    Smaa = 51,
    Letterbox = 52,
    DepthOfField = 53,
    PostprocessInitialDownsample = 54,
    CopyDepth = 55,
    DisplacementMotionBlur = 56,
    DebugShader = 57,
    MinmaxDepth = 58,
    SdsmBiasAndScale = 59,
    SdsmBiasAndScaleTextures = 60,
    ComputeShadowMapData = 61,
    ComputeLocalLightShadowMapData = 62,
    BilateralUpsample = 63,
    HealthOverlay = 64,
    LightProbeDominantLight = 65,
    LightProbeLightInstance = 66,
    Water = 67,
    LensFlare = 68,
    ScreenShader = 69,
    Scaler = 70,
    GammaControl = 71,
    SpeedtreePlacements = 72,
    Reticle = 73,
    Distortion = 74,
    WaterDebug = 75,
    ScreenAreaInput = 76,
    WaterDepthPrepass = 77,
    OverheadVisibilityMap = 78,
    ParticleCompute = 79,
    CubemapFiltering = 80,
    ParticleFastpath = 81,
    VolumetricsPass = 82,
    TemporalReprojection = 83,
    FxaaCompute = 84,
    VbCopyCompute = 85,
    UberDepth = 86,
    GearDye = 87,
    Cubemaps = 88,
    ShadowBlendWithPrevious = 89,
    DebugShadingOutput = 90,
    Ssao3d = 91,
    WaterDisplacement = 92,
    PatternBlending = 93,
    UiHdrTransform = 94,
    PlayerCenteredCascadedGrid = 95,
    SoftDeform = 96,
}

#[binread]
#[br(repr(u8))]
#[repr(u8)]
#[derive(Clone, Copy, Debug, FromPrimitive)]
pub enum TfxShaderStage {
    Pixel = 1,
    Vertex = 2,
    Geometry = 3,
    Hull = 4,
    Compute = 5,
    Domain = 6,
}

macro_rules! stage_function_match {
    ($dcs:expr, $stage:expr, $name:ident, $($arg:expr),+) => {
        paste::paste! {
            match $stage {
                TfxShaderStage::Pixel => $dcs
                    .context()
                    .[<PS $name>]($($arg, )*),
                TfxShaderStage::Vertex => $dcs
                    .context()
                    .[<VS $name>]($($arg, )*),
                TfxShaderStage::Geometry => $dcs
                    .context()
                    .[<GS $name>]($($arg, )*),
                TfxShaderStage::Hull => $dcs
                    .context()
                    .[<HS $name>]($($arg, )*),
                TfxShaderStage::Compute => $dcs
                    .context()
                    .[<CS $name>]($($arg, )*),
                TfxShaderStage::Domain => $dcs
                    .context()
                    .[<DS $name>]($($arg, )*),
            }
        }
    };
}

impl TfxShaderStage {
    /// Decodes shader stage from TFX bytecode value
    pub fn from_tfx_value(value: u8) -> anyhow::Result<TfxShaderStage> {
        Self::from_u8(value >> 5).context("Invalid shader stage index")
    }

    pub fn set_shader_resources(
        &self,
        dcs: &DeviceContextSwapchain,
        start_slot: u32,
        shader_resource_views: Option<&[Option<ID3D11ShaderResourceView>]>,
    ) {
        unsafe {
            stage_function_match!(
                dcs,
                self,
                SetShaderResources,
                start_slot,
                shader_resource_views
            );
        }
    }

    pub fn set_constant_buffers(
        &self,
        dcs: &DeviceContextSwapchain,
        start_slot: u32,
        constant_buffers: Option<&[Option<ID3D11Buffer>]>,
    ) {
        unsafe {
            stage_function_match!(dcs, self, SetConstantBuffers, start_slot, constant_buffers);
        }
    }

    pub fn set_samplers(
        &self,
        dcs: &DeviceContextSwapchain,
        start_slot: u32,
        samplers: Option<&[Option<ID3D11SamplerState>]>,
    ) {
        unsafe {
            stage_function_match!(dcs, self, SetSamplers, start_slot, samplers);
        }
    }
}
