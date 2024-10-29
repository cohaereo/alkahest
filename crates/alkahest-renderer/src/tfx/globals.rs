use alkahest_data::render_globals::{SRenderGlobals, SUnk808066ae, SUnk808067a8, SUnk8080822d};
use alkahest_pm::package_manager;
use anyhow::Context;
use destiny_pkg::TagHash;
use rustc_hash::FxHashMap;
use tiger_parse::PackageManagerExt;

use crate::{
    gpu::{texture::Texture, GpuContext, SharedGpuContext},
    loaders::technique::load_technique,
    renderer::RenderDebugView,
    tfx::{scope::TfxScope, technique::Technique},
    util::d3d::ErrorExt,
};

pub struct RenderGlobals {
    pub scopes: GlobalScopes,
    pub pipelines: GlobalPipelines,

    pub textures: GlobalTextures,

    pub unk34: SUnk8080822d,
}

impl RenderGlobals {
    pub fn load(gctx: SharedGpuContext) -> anyhow::Result<Self> {
        let data: SRenderGlobals = package_manager().read_named_tag_struct("render_globals")?;
        let globs = &data.unk8.first().context("No render globals found")?.unk8.0;

        Ok(Self {
            scopes: GlobalScopes::load(gctx.clone(), globs),
            pipelines: GlobalPipelines::load(gctx.clone(), globs),
            textures: GlobalTextures::load(&gctx, &globs.unk30),
            unk34: globs.unk34.0.clone(),
        })
    }
}

pub struct GlobalTextures {
    pub specular_tint_lookup: Texture,
    pub specular_lobe_lookup: Texture,
    pub specular_lobe_3d_lookup: Texture,
    pub iridescence_lookup: Texture,
}

impl GlobalTextures {
    pub fn load(gctx: &GpuContext, data: &SUnk808066ae) -> Self {
        Self {
            specular_tint_lookup: Texture::load(
                &gctx.device,
                data.specular_tint_lookup_texture.into(),
            )
            .unwrap(),
            specular_lobe_lookup: Texture::load(
                &gctx.device,
                data.specular_lobe_lookup_texture.into(),
            )
            .unwrap(),
            specular_lobe_3d_lookup: Texture::load(
                &gctx.device,
                data.specular_lobe_3d_lookup_texture.into(),
            )
            .unwrap(),
            iridescence_lookup: Texture::load(&gctx.device, data.iridescence_lookup_texture.into())
                .unwrap(),
        }
    }
}

macro_rules! tfx_global_scopes {
    ($($name:ident),*) => {
        pub struct GlobalScopes {
            $(
                pub $name: TfxScope,
            )*
        }


        impl GlobalScopes {
            pub fn load(gctx: SharedGpuContext, globals: &SUnk808067a8) -> Self {
                let scopes: FxHashMap<String, TagHash> = globals.scopes.iter().map(|p| (p.name.to_string(), p.scope)).collect();

                Self {
                    $(
                        $name: TfxScope::load(
                            package_manager()
                                .read_tag_struct(scopes[stringify!($name)])
                                .unwrap_or_else(|_| panic!("Failed to read scope {}", stringify!($name))),
                            gctx.clone(),
                        )
                        .expect("Failed to load scope"),
                    )*
                }
            }
        }
    };
}

tfx_global_scopes! {
    frame, view, rigid_model, editor_mesh, editor_terrain,
    cui_view, cui_object, skinning, speedtree, chunk_model,
    decal, instances, speedtree_lod_drawcall_data, transparent,
    transparent_advanced, sdsm_bias_and_scale_textures, terrain,
    postprocess, cui_bitmap, cui_standard, ui_font, cui_hud,
    particle_transforms, particle_location_metadata, cubemap_volume,
    gear_plated_textures, gear_dye_0, gear_dye_1, gear_dye_2,
    gear_dye_decal, generic_array, gear_dye_skin, gear_dye_lips,
    gear_dye_hair, gear_dye_facial_layer_0_mask, gear_dye_facial_layer_0_material,
    gear_dye_facial_layer_1_mask, gear_dye_facial_layer_1_material,
    player_centered_cascaded_grid, gear_dye_012, color_grading_ubershader
}

macro_rules! tfx_global_pipelines {
    ($($name:ident),*) => {
        #[allow(non_snake_case)]
        pub struct GlobalPipelines {
            $(
                pub $name: Box<Technique>,
            )*
        }


        impl GlobalPipelines {
            pub fn load(gctx: SharedGpuContext, globals: &SUnk808067a8) -> Self {
                let techniques: FxHashMap<String, TagHash> = globals.unk20.iter().map(|p| (p.name.to_string(), p.technique)).collect();

                Self {
                    $(
                        $name: Box::new(
                            load_technique(gctx.clone(), techniques[stringify!($name)])
                                .map_err(|e| e.with_d3d_error(&gctx))
                                .unwrap_or_else(|_| panic!("Failed to read global pipeline technique {}", stringify!($name))),
                        ),
                    )*
                }
            }
        }
    };
}

tfx_global_pipelines! {
    // Shading
    clear_color_2_mrt,
    deferred_shading,
    deferred_shading_no_atm,
    global_lighting,
    global_lighting_ambient_only_and_shading,
    global_lighting_ambient_only,
    // global_lighting_and_shading_gel,
    global_lighting_and_shading,
    // global_lighting_gel,
    // global_lighting_masked_sun_and_shading_gel,
    // global_lighting_masked_sun_and_shading,
    // global_lighting_masked_sun_gel,
    // global_lighting_masked_sun,
    final_combine_no_film_curve,
    final_combine,

    // Post
    hdao,
    apply_ssao_to_light_buffers,
    ssao_bilateral_filter,
    ssao_compute_ao_3D_ps,
    fxaa,
    fxaa_noise,

    // Atmosphere/Sky
    hemisphere_sky_color_generate,
    sky,
    sky_lookup_generate_far,
    sky_lookup_generate_near,
    sky_hemisphere_copy_and_tint,
    sky_hemisphere_copy_frustum,
    sky_hemisphere_downsample_filter_ggx,
    sky_hemisphere_filter_cosine,
    sky_hemisphere_seed_inscattering,
    sky_hemisphere_spherical_blur,
    sky_direction_lookup_generate,
    sky_generate_sky_mask,
    atmo_depth_angle_density_lookup_generate,

    // Cubemap variants
    cubemap_apply_cube_alpha_off_probes_off_relighting_off, cubemap_apply_cube_alpha_off_probes_off_relighting_on,
    cubemap_apply_cube_alpha_off_probes_on_relighting_off, cubemap_apply_cube_alpha_off_probes_on_relighting_on,
    cubemap_apply_cube_alpha_on_probes_off_relighting_off, cubemap_apply_cube_alpha_on_probes_off_relighting_on,
    cubemap_apply_cube_alpha_on_probes_on_relighting_off, cubemap_apply_cube_alpha_on_probes_on_relighting_on,
    cubemap_apply_cube_sphere_alpha_off_probes_off_relighting_off, cubemap_apply_cube_sphere_alpha_off_probes_off_relighting_on,
    cubemap_apply_cube_sphere_alpha_off_probes_on_relighting_off, cubemap_apply_cube_sphere_alpha_off_probes_on_relighting_on,
    cubemap_apply_cube_sphere_alpha_on_probes_off_relighting_off, cubemap_apply_cube_sphere_alpha_on_probes_off_relighting_on,
    cubemap_apply_cube_sphere_alpha_on_probes_on_relighting_off, cubemap_apply_cube_sphere_alpha_on_probes_on_relighting_on,
    cubemap_apply_sphere_alpha_off_probes_off_relighting_off, cubemap_apply_sphere_alpha_off_probes_off_relighting_on,
    cubemap_apply_sphere_alpha_off_probes_on_relighting_off, cubemap_apply_sphere_alpha_off_probes_on_relighting_on,
    cubemap_apply_sphere_alpha_on_probes_off_relighting_off, cubemap_apply_sphere_alpha_on_probes_off_relighting_on,
    cubemap_apply_sphere_alpha_on_probes_on_relighting_off, cubemap_apply_sphere_alpha_on_probes_on_relighting_on,
    cubemap_apply_parall_cube_alpha_off_probes_off_relighting_off, cubemap_apply_parall_cube_alpha_off_probes_off_relighting_on,
    cubemap_apply_parall_cube_alpha_off_probes_on_relighting_off, cubemap_apply_parall_cube_alpha_off_probes_on_relighting_on,
    cubemap_apply_parall_cube_alpha_on_probes_off_relighting_off, cubemap_apply_parall_cube_alpha_on_probes_off_relighting_on,
    cubemap_apply_parall_cube_alpha_on_probes_on_relighting_off, cubemap_apply_parall_cube_alpha_on_probes_on_relighting_on,
    cubemap_apply_parall_cube_sphere_alpha_off_probes_off_relighting_off, cubemap_apply_parall_cube_sphere_alpha_off_probes_off_relighting_on,
    cubemap_apply_parall_cube_sphere_alpha_off_probes_on_relighting_off, cubemap_apply_parall_cube_sphere_alpha_off_probes_on_relighting_on,
    cubemap_apply_parall_cube_sphere_alpha_on_probes_off_relighting_off, cubemap_apply_parall_cube_sphere_alpha_on_probes_off_relighting_on,
    cubemap_apply_parall_cube_sphere_alpha_on_probes_on_relighting_off, cubemap_apply_parall_cube_sphere_alpha_on_probes_on_relighting_on,
    cubemap_apply_parall_sphere_alpha_off_probes_off_relighting_off, cubemap_apply_parall_sphere_alpha_off_probes_off_relighting_on,
    cubemap_apply_parall_sphere_alpha_off_probes_on_relighting_off, cubemap_apply_parall_sphere_alpha_off_probes_on_relighting_on,
    cubemap_apply_parall_sphere_alpha_on_probes_off_relighting_off, cubemap_apply_parall_sphere_alpha_on_probes_off_relighting_on,
    cubemap_apply_parall_sphere_alpha_on_probes_on_relighting_off, cubemap_apply_parall_sphere_alpha_on_probes_on_relighting_on,

    cubemap_apply_override_hdr_with_sky_mask,
    cubemap_apply_override_opaque,
    cubemap_apply_override_with_sky_mask,
    cubemap_apply_sky_ambient_occlusion,
    cubemap_apply_sky_copy_ao_with_skybox_fade,
    cubemap_apply_sky_copy_ao,
    cubemap_apply_sky_specular_occlusion,
    cubemap_apply,

    // Debug
    debug_ambient_occlusion_source_color,
    debug_ambient_occlusion,
    debug_color_per_ao_status,
    debug_color_per_draw_call,
    debug_color_per_instance,
    debug_colored_overcoat_id,
    debug_colored_overcoat,
    debug_depth_edges,
    debug_depth_gradient,
    debug_depth_walkable,
    debug_depth,
    debug_diffuse_color,
    debug_diffuse_ibl,
    debug_diffuse_light,
    debug_diffuse_only,
    debug_emissive_intensity,
    debug_emissive_luminance,
    debug_emissive,
    debug_force_pbr_valid,
    debug_gbuffer_validation,
    debug_grey_diffuse,
    debug_local_ambient_occlusion,
    debug_metalness,
    debug_normal_compression,
    debug_normal_edges,
    debug_shader_chunked,
    debug_shader_cloth_skinned,
    debug_shader_cloth,
    debug_shader_decal,
    debug_shader_decorator,
    debug_shader_dq_skinned,
    debug_shader_lb_skinned,
    debug_shader_per_bone_scaled_lb_skinned,
    debug_shader_rigid_model,
    debug_shader_road_decal,
    debug_shader_terrain_thumbnail,
    debug_shader_terrain,
    debug_source_color_luminance,
    debug_source_color,
    debug_specular_color,
    debug_specular_ibl,
    debug_specular_light,
    debug_specular_occlusion,
    debug_specular_only,
    debug_specular_smoothness,
    debug_specular_tint,
    debug_specular_lobe,
    debug_texture_ao,
    debug_transmission,
    debug_valid_layered_metalness,
    debug_valid_smoothness_heatmap,
    debug_valid_source_color_brightness,
    debug_valid_source_color_saturation,
    debug_vertex_color,
    debug_world_normal,

    // Feature renderer debug
    debug_cubemap_diffuse_opacity,
    debug_cubemap_diffuse_probes_alpha,
    debug_cubemap_diffuse_probes,
    debug_cubemap_overdraw,
    debug_cubemap_specular_opacity
}

#[derive(PartialEq)]
pub enum CubemapShape {
    Cube = 0,
    CubeSphere = 1,
    Sphere = 2,
    ParallCube = 3,
    ParallCubeSphere = 4,
    ParallSphere = 5,
}

impl GlobalPipelines {
    pub fn get_specialized_cubemap_pipeline(
        &self,
        shape: CubemapShape,
        alpha: bool,
        probes: bool,
        relighting: bool,
    ) -> &Technique {
        let pipeline_list = [
            // Cube
            &self.cubemap_apply_cube_alpha_off_probes_off_relighting_off,
            &self.cubemap_apply_cube_alpha_off_probes_off_relighting_on,
            &self.cubemap_apply_cube_alpha_off_probes_on_relighting_off,
            &self.cubemap_apply_cube_alpha_off_probes_on_relighting_on,
            &self.cubemap_apply_cube_alpha_on_probes_off_relighting_off,
            &self.cubemap_apply_cube_alpha_on_probes_off_relighting_on,
            &self.cubemap_apply_cube_alpha_on_probes_on_relighting_off,
            &self.cubemap_apply_cube_alpha_on_probes_on_relighting_on,
            // CubeSphere
            &self.cubemap_apply_cube_sphere_alpha_off_probes_off_relighting_off,
            &self.cubemap_apply_cube_sphere_alpha_off_probes_off_relighting_on,
            &self.cubemap_apply_cube_sphere_alpha_off_probes_on_relighting_off,
            &self.cubemap_apply_cube_sphere_alpha_off_probes_on_relighting_on,
            &self.cubemap_apply_cube_sphere_alpha_on_probes_off_relighting_off,
            &self.cubemap_apply_cube_sphere_alpha_on_probes_off_relighting_on,
            &self.cubemap_apply_cube_sphere_alpha_on_probes_on_relighting_off,
            &self.cubemap_apply_cube_sphere_alpha_on_probes_on_relighting_on,
            // Sphere
            &self.cubemap_apply_sphere_alpha_off_probes_off_relighting_off,
            &self.cubemap_apply_sphere_alpha_off_probes_off_relighting_on,
            &self.cubemap_apply_sphere_alpha_off_probes_on_relighting_off,
            &self.cubemap_apply_sphere_alpha_off_probes_on_relighting_on,
            &self.cubemap_apply_sphere_alpha_on_probes_off_relighting_off,
            &self.cubemap_apply_sphere_alpha_on_probes_off_relighting_on,
            &self.cubemap_apply_sphere_alpha_on_probes_on_relighting_off,
            &self.cubemap_apply_sphere_alpha_on_probes_on_relighting_on,
            // ParallCube
            &self.cubemap_apply_parall_cube_alpha_off_probes_off_relighting_off,
            &self.cubemap_apply_parall_cube_alpha_off_probes_off_relighting_on,
            &self.cubemap_apply_parall_cube_alpha_off_probes_on_relighting_off,
            &self.cubemap_apply_parall_cube_alpha_off_probes_on_relighting_on,
            &self.cubemap_apply_parall_cube_alpha_on_probes_off_relighting_off,
            &self.cubemap_apply_parall_cube_alpha_on_probes_off_relighting_on,
            &self.cubemap_apply_parall_cube_alpha_on_probes_on_relighting_off,
            &self.cubemap_apply_parall_cube_alpha_on_probes_on_relighting_on,
            // ParallCubeSphere
            &self.cubemap_apply_parall_cube_sphere_alpha_off_probes_off_relighting_off,
            &self.cubemap_apply_parall_cube_sphere_alpha_off_probes_off_relighting_on,
            &self.cubemap_apply_parall_cube_sphere_alpha_off_probes_on_relighting_off,
            &self.cubemap_apply_parall_cube_sphere_alpha_off_probes_on_relighting_on,
            &self.cubemap_apply_parall_cube_sphere_alpha_on_probes_off_relighting_off,
            &self.cubemap_apply_parall_cube_sphere_alpha_on_probes_off_relighting_on,
            &self.cubemap_apply_parall_cube_sphere_alpha_on_probes_on_relighting_off,
            &self.cubemap_apply_parall_cube_sphere_alpha_on_probes_on_relighting_on,
            // ParallSphere
            &self.cubemap_apply_parall_sphere_alpha_off_probes_off_relighting_off,
            &self.cubemap_apply_parall_sphere_alpha_off_probes_off_relighting_on,
            &self.cubemap_apply_parall_sphere_alpha_off_probes_on_relighting_off,
            &self.cubemap_apply_parall_sphere_alpha_off_probes_on_relighting_on,
            &self.cubemap_apply_parall_sphere_alpha_on_probes_off_relighting_off,
            &self.cubemap_apply_parall_sphere_alpha_on_probes_off_relighting_on,
            &self.cubemap_apply_parall_sphere_alpha_on_probes_on_relighting_off,
            &self.cubemap_apply_parall_sphere_alpha_on_probes_on_relighting_on,
        ];

        let shape_index = shape as usize;

        let alpha_index = if alpha { 1 } else { 0 };
        let probes_index = if probes { 1 } else { 0 };
        let relighting_index = if relighting { 1 } else { 0 };

        pipeline_list[shape_index * 8 + alpha_index * 4 + probes_index * 2 + relighting_index]
    }

    pub fn get_debug_view_pipeline(&self, view: RenderDebugView) -> &Technique {
        match view {
            RenderDebugView::None => &self.final_combine,
            RenderDebugView::NoFilmCurve => &self.final_combine_no_film_curve,
            RenderDebugView::GbufferValidation => &self.debug_gbuffer_validation,
            RenderDebugView::SourceColor => &self.debug_source_color,
            RenderDebugView::Normal => &self.debug_world_normal,
            RenderDebugView::NormalEdges => &self.debug_normal_edges,
            RenderDebugView::Metalness => &self.debug_metalness,
            RenderDebugView::AmbientOcclusion => &self.debug_ambient_occlusion,
            RenderDebugView::TextureAo => &self.debug_texture_ao,
            RenderDebugView::Transmission => &self.debug_transmission,

            RenderDebugView::ColoredOvercoatId => &self.debug_colored_overcoat_id,
            RenderDebugView::ColoredOvercoat => &self.debug_colored_overcoat,

            RenderDebugView::DiffuseColor => &self.debug_diffuse_color,
            RenderDebugView::DiffuseLight => &self.debug_diffuse_light,

            RenderDebugView::SpecularColor => &self.debug_specular_color,
            RenderDebugView::SpecularLight => &self.debug_specular_light,
            RenderDebugView::SpecularOcclusion => &self.debug_specular_occlusion,
            RenderDebugView::SpecularSmoothness => &self.debug_specular_smoothness,
            RenderDebugView::SpecularOnly => &self.debug_specular_only,

            RenderDebugView::Emissive => &self.debug_emissive,
            RenderDebugView::EmissiveIntensity => &self.debug_emissive_intensity,
            RenderDebugView::EmissiveLuminance => &self.debug_emissive_luminance,

            RenderDebugView::GreyDiffuse => &self.debug_grey_diffuse,

            RenderDebugView::Depth => &self.debug_depth,
            RenderDebugView::DepthEdges => &self.debug_depth_edges,
            RenderDebugView::DepthGradient => &self.debug_depth_gradient,
            RenderDebugView::DepthWalkable => &self.debug_depth_walkable,

            RenderDebugView::ValidLayeredMetalness => &self.debug_valid_layered_metalness,
            RenderDebugView::ValidSmoothnessHeatmap => &self.debug_valid_smoothness_heatmap,
            RenderDebugView::ValidSourceColorBrightness => {
                &self.debug_valid_source_color_brightness
            }
            RenderDebugView::ValidSourceColorSaturation => {
                &self.debug_valid_source_color_saturation
            }
        }
    }
}
