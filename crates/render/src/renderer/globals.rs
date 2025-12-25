use std::{collections::HashMap, sync::Arc};

use alkahest_data::tfx::{
    features::cubemap::CubemapShape,
    render_globals::{
        SRenderGlobalLookupTextures, SRenderGlobals, SRenderGlobalsData,
        SRenderGlobalsGlobalChannels,
    },
};
use anyhow::Context;
use tiger_parse::PackageManagerExt;
use tiger_pkg::{package_manager, TagHash};

use crate::{
    asset::texture::Texture,
    tfx::{scope::Scope, technique::Technique},
    Gpu,
};

pub struct RenderGlobals {
    pub scopes: GlobalScopes,
    pub pipelines: GlobalPipelines,

    pub textures: GlobalTextures,
    pub channels: SRenderGlobalsGlobalChannels,
    // pub unk34: SUnk8080822d,
}

impl RenderGlobals {
    pub const CHANNEL_SUN_LIGHT_DIRECTION: u32 = 0x5C579DFA;

    pub fn load(gpu: &Arc<Gpu>) -> anyhow::Result<Self> {
        let data: SRenderGlobals = package_manager().read_named_tag_struct("render_globals")?;
        let globs = &data.unk8.first().context("No render globals found")?.unk8.0;

        Ok(Self {
            scopes: GlobalScopes::load(gpu, globs),
            pipelines: GlobalPipelines::load(gpu, globs),
            textures: GlobalTextures::load(gpu, &globs.unk30)?,
            channels: globs.global_channels.0.clone(),
            // unk34: globs.unk34.0.clone(),
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
    pub fn load(gpu: &Gpu, data: &SRenderGlobalLookupTextures) -> anyhow::Result<Self> {
        Ok(Self {
            specular_tint_lookup: Texture::load(gpu, data.specular_tint_lookup_texture)?,
            specular_lobe_lookup: Texture::load(gpu, data.specular_lobe_lookup_texture)?,
            specular_lobe_3d_lookup: Texture::load(gpu, data.specular_lobe_3d_lookup_texture)?,
            iridescence_lookup: Texture::load(gpu, data.iridescence_lookup_texture)?,
        })
    }
}

macro_rules! tfx_global_scopes {
    ($($name:ident),*) => {
        pub struct GlobalScopes {
            $(
                pub $name: Box<Scope>,
            )*
        }


        impl GlobalScopes {
            pub fn load(gpu: &Arc<Gpu>, globals: &SRenderGlobalsData) -> Self {
                let scopes: HashMap<String, TagHash> = globals.scopes.iter().map(|p| (p.name.to_string(), p.scope)).collect();

                Self {
                    $(
                        $name: Box::new(Scope::load(
                            gpu,
                            *scopes.get(stringify!($name))
                                .expect(&format!("Scope {} does not exist", stringify!($name))),
                        )
                        .expect("Failed to load scope")),
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
    gear_plated_textures, generic_array
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
            pub fn load(gpu: &Arc<Gpu>, globals: &SRenderGlobalsData) -> Self {
                let techniques: HashMap<String, TagHash> = globals.pipelines.iter().map(|p| (p.name.to_string(), p.technique)).collect();

                Self {
                    $(
                        $name: Box::new(
                            Technique::load(
                                gpu,
                                *techniques.get(stringify!($name))
                                    .expect(&format!("Technique {} does not exist", stringify!($name)))
                            )
                            .unwrap_or_else(|e| panic!("Failed to read global pipeline technique {}: {e:?}", stringify!($name))),
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
    global_lighting_and_shading,
    global_lighting_and_shading_gel,
    final_combine_no_film_curve,
    final_combine,

    // Post
    hdao,
    apply_ssao_to_light_buffers,
    ssao_bilateral_filter,
    // ssao_compute_ao_3D_ps,
    fxaa,
    fxaa_noise,
    autoexposure_sample_columns,

    // Utility
    copy_texture_bilinear,
    copy_texture_bilinear_tiled,

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

    cubemap_apply_sky_copy_ao,

    debug_cubemap_diffuse_probes,
    debug_source_color,
    debug_specular_smoothness,
    debug_metalness,
    debug_texture_ao,
    debug_ambient_occlusion,
    debug_emissive,
    debug_emissive_intensity,
    debug_transmission,
    debug_colored_overcoat_id,
    debug_depth_edges,
    debug_world_normal,
    debug_diffuse_light,
    debug_specular_light,

    // LUT3D variants
    screen_area_global_lut3d_distort,
    screen_area_global_lut3d_distort_hdr,
    screen_area_global_lut3d_distort_noise,
    screen_area_global_lut3d_distort_noise_hdr,
    screen_area_global_lut3d,
    screen_area_global_lut3d_hdr,
    screen_area_global_lut3d_noise,
    screen_area_global_lut3d_noise_hdr,
    screen_area_global_lut3d_no_tonemap,

    downsample_depth_buffer, uber_depth_default, downsample_max_min_avg_no_swizzle,

    bloom_initial_downsample_block_2x2, downsample_block_2x2_with_nan_kill,

    volumetrics_upres_1,

    water_sky_color_generate,
    water_reflection_healing,
    water_reflection_resolve,
    water_reflection_uv_healing
}

impl GlobalPipelines {
    pub fn get_specialized_cubemap_pipeline(
        &self,
        shape: CubemapShape,
        alpha: bool,
        probes: bool,
        relighting: bool,
        parallax: bool,
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
            // Sphere
            &self.cubemap_apply_sphere_alpha_off_probes_off_relighting_off,
            &self.cubemap_apply_sphere_alpha_off_probes_off_relighting_on,
            &self.cubemap_apply_sphere_alpha_off_probes_on_relighting_off,
            &self.cubemap_apply_sphere_alpha_off_probes_on_relighting_on,
            &self.cubemap_apply_sphere_alpha_on_probes_off_relighting_off,
            &self.cubemap_apply_sphere_alpha_on_probes_off_relighting_on,
            &self.cubemap_apply_sphere_alpha_on_probes_on_relighting_off,
            &self.cubemap_apply_sphere_alpha_on_probes_on_relighting_on,
            // CubeSphere
            &self.cubemap_apply_cube_sphere_alpha_off_probes_off_relighting_off,
            &self.cubemap_apply_cube_sphere_alpha_off_probes_off_relighting_on,
            &self.cubemap_apply_cube_sphere_alpha_off_probes_on_relighting_off,
            &self.cubemap_apply_cube_sphere_alpha_off_probes_on_relighting_on,
            &self.cubemap_apply_cube_sphere_alpha_on_probes_off_relighting_off,
            &self.cubemap_apply_cube_sphere_alpha_on_probes_off_relighting_on,
            &self.cubemap_apply_cube_sphere_alpha_on_probes_on_relighting_off,
            &self.cubemap_apply_cube_sphere_alpha_on_probes_on_relighting_on,
            // ParallCube
            &self.cubemap_apply_parall_cube_alpha_off_probes_off_relighting_off,
            &self.cubemap_apply_parall_cube_alpha_off_probes_off_relighting_on,
            &self.cubemap_apply_parall_cube_alpha_off_probes_on_relighting_off,
            &self.cubemap_apply_parall_cube_alpha_off_probes_on_relighting_on,
            &self.cubemap_apply_parall_cube_alpha_on_probes_off_relighting_off,
            &self.cubemap_apply_parall_cube_alpha_on_probes_off_relighting_on,
            &self.cubemap_apply_parall_cube_alpha_on_probes_on_relighting_off,
            &self.cubemap_apply_parall_cube_alpha_on_probes_on_relighting_on,
            // ParallSphere
            &self.cubemap_apply_parall_sphere_alpha_off_probes_off_relighting_off,
            &self.cubemap_apply_parall_sphere_alpha_off_probes_off_relighting_on,
            &self.cubemap_apply_parall_sphere_alpha_off_probes_on_relighting_off,
            &self.cubemap_apply_parall_sphere_alpha_off_probes_on_relighting_on,
            &self.cubemap_apply_parall_sphere_alpha_on_probes_off_relighting_off,
            &self.cubemap_apply_parall_sphere_alpha_on_probes_off_relighting_on,
            &self.cubemap_apply_parall_sphere_alpha_on_probes_on_relighting_off,
            &self.cubemap_apply_parall_sphere_alpha_on_probes_on_relighting_on,
            // ParallCubeSphere
            &self.cubemap_apply_parall_cube_sphere_alpha_off_probes_off_relighting_off,
            &self.cubemap_apply_parall_cube_sphere_alpha_off_probes_off_relighting_on,
            &self.cubemap_apply_parall_cube_sphere_alpha_off_probes_on_relighting_off,
            &self.cubemap_apply_parall_cube_sphere_alpha_off_probes_on_relighting_on,
            &self.cubemap_apply_parall_cube_sphere_alpha_on_probes_off_relighting_off,
            &self.cubemap_apply_parall_cube_sphere_alpha_on_probes_off_relighting_on,
            &self.cubemap_apply_parall_cube_sphere_alpha_on_probes_on_relighting_off,
            &self.cubemap_apply_parall_cube_sphere_alpha_on_probes_on_relighting_on,
        ];

        let shape_index = shape as usize;
        let alpha_index = alpha as usize;
        let probes_index = probes as usize;
        let relighting_index = relighting as usize;
        let parallax_index = parallax as usize;

        pipeline_list[parallax_index * 24
            + shape_index * 8
            + alpha_index * 4
            + probes_index * 2
            + relighting_index]
    }

    pub fn get_specialized_lut3d_pipeline(
        &self,
        distort: bool,
        hdr: bool,
        noise: bool,
    ) -> &Technique {
        let pipeline_list = [
            // No Distort
            &self.screen_area_global_lut3d,
            &self.screen_area_global_lut3d_hdr,
            &self.screen_area_global_lut3d_noise,
            &self.screen_area_global_lut3d_noise_hdr,
            // Distort
            &self.screen_area_global_lut3d_distort,
            &self.screen_area_global_lut3d_distort_hdr,
            &self.screen_area_global_lut3d_distort_noise,
            &self.screen_area_global_lut3d_distort_noise_hdr,
        ];

        let distort_index = distort as usize;
        let hdr_index = hdr as usize;
        let noise_index = noise as usize;

        pipeline_list[distort_index * 4 + noise_index * 2 + hdr_index]
    }
}
