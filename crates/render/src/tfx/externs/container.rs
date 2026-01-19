use std::any::TypeId;

use alkahest_data::{hash::fnv1, tfx::ExternIndex};
use glam::Vec4;

use super::definitions::*;
use crate::{
    Renderer,
    renderer::globals::RenderGlobals,
    tfx::externs::{
        Extern, ExternAccessor,
        macros::{extern_container, local_extern_container},
    },
};

extern_container! {
    frame: Frame,
    view: View,
    deferred: Deferred,
    deferred_light: DeferredLight,
    deferred_uber_light: DeferredUberLight,
    deferred_shadow: DeferredShadow,
    atmosphere: Atmosphere,
    rigid_model: RigidModel,
    editor_mesh_material: EditorMeshMaterial,
    editor_decal: EditorDecal,
    editor_terrain: EditorTerrain,
    editor_terrain_patch: EditorTerrainPatch,
    editor_terrain_debug: EditorTerrainDebug,
    simple_geometry: SimpleGeometry,
    cui_object: CuiObject,
    cui_video: CuiVideo,
    cui_standard: CuiStandard,
    cui_screenspace_boxes: CuiScreenspaceBoxes,
    texture_visualizer: TextureVisualizer,
    generic: Generic,
    particle: Particle,
    particle_debug: ParticleDebug,
    screen_area: ScreenArea,
    msaa: Msaa,
    hdao: Hdao,
    downsample_texture_generic: DownsampleTextureGeneric,
    downsample_depth: DownsampleDepth,
    ssao: Ssao,
    postprocess: Postprocess,
    transparent: Transparent,
    vignette: Vignette,
    global_lighting: GlobalLighting,
    shadow_mask: ShadowMask,
    object_effect: ObjectEffect,
    decal: Decal,
    decal_set_transform: DecalSetTransform,
    dynamic_decal: DynamicDecal,
    decorator_wind: DecoratorWind,
    volume_fog: VolumeFog,
    fxaa: Fxaa,
    smaa: Smaa,
    letterbox: Letterbox,
    depth_of_field: DepthOfField,
    postprocess_initial_downsample: PostprocessInitialDownsample,
    displacement_motion_blur: DisplacementMotionBlur,
    debug_shader: DebugShader,
    minmax_depth: MinmaxDepth,
    sdsm_bias_and_scale: SdsmBiasAndScale,
    sdsm_bias_and_scale_textures: SdsmBiasAndScaleTextures,
    compute_shadow_map_data: ComputeShadowMapData,
    compute_local_light_shadow_map_data: ComputeLocalLightShadowMapData,
    bilateral_upsample: BilateralUpsample,
    health_overlay: HealthOverlay,
    light_probe_dominant_light: LightProbeDominantLight,
    light_probe_light_instance: LightProbeLightInstance,
    water: Water,
    lens_flare: LensFlare,
    screen_shader: ScreenShader,
    scaler: Scaler,
    gamma_control: GammaControl,
    speedtree_placements: SpeedtreePlacements,
    reticle: Reticle,
    distortion: Distortion,
    water_depth_prepass: WaterDepthPrepass,
    particle_compute: ParticleCompute,
    cubemap_filtering: CubemapFiltering,
    volumetrics_pass: VolumetricsPass,
    temporal_reprojection: TemporalReprojection,
    uber_depth: UberDepth,
    cubemaps: Cubemaps,
    shadow_blend_with_previous: ShadowBlendWithPrevious,
    debug_shading_output: DebugShadingOutput,
    water_displacement: WaterDisplacement,
    pattern_blending: PatternBlending,
    ui_hdr_transform: UiHdrTransform,
    player_centered_cascaded_grid: PlayerCenteredCascadedGrid,
    soft_deform: SoftDeform
}

impl Externs {
    pub fn get_global_channel_by_name(&self, name: &str) -> Vec4 {
        self.get_global_channel_by_id(fnv1(name))
    }

    pub fn get_global_channel_by_id(&self, id: u32) -> Vec4 {
        self.global_ids
            .iter()
            .position(|i| *i == id)
            .map(|pos| self.globals[pos])
            .unwrap_or(Vec4::ONE)
    }

    /// Sets the value of the given global channel by ID
    /// Returns `Some` with the previous value if the channel exists, `None` otherwise
    pub fn set_global_channel_by_id(&mut self, id: u32, v: Vec4) -> Option<Vec4> {
        if let Some(pos) = self.global_ids.iter().position(|i| *i == id) {
            Some(std::mem::replace(&mut self.globals[pos], v))
        } else {
            None
        }
    }

    /// Sets the value of the given global channel by name, hashing the name to get its ID
    /// Returns `Some` with the previous value if the channel exists, `None` otherwise
    pub fn set_global_channel_by_name(&mut self, name: &str, v: Vec4) -> Option<Vec4> {
        self.set_global_channel_by_id(fnv1(name), v)
    }

    /// Resets every global channel to their default value, as defined by render globals
    pub fn reset_global_channels(&mut self) {
        let globals = &Renderer::instance().globals;
        self.globals[..globals.channels.default_values.len()]
            .copy_from_slice(&globals.channels.default_values);
    }
}

local_extern_container! {
    rigid_model: RigidModel,
    speedtree_placements: SpeedtreePlacements,
    simple_geometry: SimpleGeometry,
    deferred_light: DeferredLight,
    volume_fog: VolumeFog,
    deferred_shadow: DeferredShadow
}

const GLOBAL_CHANNEL_NAMES: &[&str] = &[
    "autoexposure_zero_bias",
    "autoexposure_bias",
    "autoexposure_max_approach_speed",
    "autoexposure_adjust_speed",
    "autoexposure_frame_delay",
    "autoexposure_min_stops",
    "autoexposure_max_stops",
    "sun_intensity",
    "sun_color",
    "sun_glow_color",
    "sun_glow_shape",
    "sun_glow_intensity",
    "sky_sun_glow_shape",
    "sky_sun_glow_intensity",
    "fog_density",
    "fog_density_lookup_start",
    "fog_density_lookup_end",
    "fog_start_height",
    "fog_height_falloff",
    "fog_decay_color",
    "fog_decay_scale",
    "layered_fog_density",
    "layered_fog_falloff",
    "layered_fog_start_height",
    "skybox_sun_intensity",
    "skybox_sun_color",
    "skybox_up_ambient_color",
    "skybox_up_ambient_intensity",
    "skybox_down_ambient_color",
    "skybox_down_ambient_intensity",
    "sky_color_override",
    "sky_snapshot_rotation",
    "sky_snapshot_intensity",
    "cubemap_splash_space_in",
    "ao_ambient_weight",
    "ao_sun_weight",
    "up_ambient_color",
    "up_ambient_intensity",
    "up_ambient_sharpness",
    "down_ambient_color",
    "down_ambient_intensity",
    "down_ambient_sharpness",
    "sun_transmission_color",
    "sun_shadow_intensity",
    "sun_transmission_intensity",
    "duplex_involved",
    "sun_light_direction",
    "sun_atmosphere_direction",
    "sun_ambient_direction",
    "sun_track_direction",
    "sun_or_moon",
    "dc_intensity",
    "dc_color",
    "fx_channel_01",
    "fx_channel_02",
    "fx_channel_03",
    "fx_channel_04",
    "fx_weather_01",
    "fx_weather_02",
    "global_weather_state",
    "global_weather_wind_direction",
    "global_weather_wind_speed",
    "character_lighting_scale_ao",
    "character_lighting_scale_emissive",
    "sky_chromatic_bake_end",
    "screenspace_reflection_intensity",
    "global_ambient_intensity",
    "global_cubemap_intensity",
    "global_cubemap_diffuse_intensity",
    "global_cubemap_down_color",
    "sun_direct_intensity",
    "cubemap_relighting_sky_intensity",
    "cubemap_bounce_scale",
    "cubemap_sky_intensity",
    "sun_tunnel_hash5",
    "sun_tunnel_hash4",
    "sun_tunnel_hash7",
    "reticle_location",
    "fx_cinematics_01",
    "fx_cinematics_02",
    "fx_cinematics_03",
    "fx_cinematics_04",
    "fx_cinematics_05",
    "fx_cinematics_06",
    "sun_tunnel_hash6",
    "sun_tunnel_hash1",
    "sun_tunnel_hash0",
    "sun_tunnel_hash3",
    "sun_tunnel_hash2",
    "flashlight_intensity",
    "flashlight_color",
];

pub fn get_global_channel_name(hash: u32) -> Option<&'static str> {
    GLOBAL_CHANNEL_NAMES
        .iter()
        .copied()
        .find(|&name| fnv1(name) == hash)
}
