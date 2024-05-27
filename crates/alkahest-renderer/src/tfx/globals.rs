use std::mem::MaybeUninit;

use alkahest_data::render_globals::{SRenderGlobals, SUnk808066ae, SUnk808067a8, SUnk8080822d};
use alkahest_pm::package_manager;
use anyhow::Context;
use field_access::FieldAccess;
use itertools::Itertools;
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

#[derive(FieldAccess)]
pub struct GlobalScopes {
    pub frame: TfxScope,
    pub view: TfxScope,
    pub rigid_model: TfxScope,
    pub editor_mesh: TfxScope,
    pub editor_terrain: TfxScope,
    pub cui_view: TfxScope,
    pub cui_object: TfxScope,
    pub skinning: TfxScope,
    pub speedtree: TfxScope,
    pub chunk_model: TfxScope,
    pub decal: TfxScope,
    pub instances: TfxScope,
    pub speedtree_lod_drawcall_data: TfxScope,
    pub transparent: TfxScope,
    pub transparent_advanced: TfxScope,
    pub sdsm_bias_and_scale_textures: TfxScope,
    pub terrain: TfxScope,
    pub postprocess: TfxScope,
    pub cui_bitmap: TfxScope,
    pub cui_standard: TfxScope,
    pub ui_font: TfxScope,
    pub cui_hud: TfxScope,
    pub particle_transforms: TfxScope,
    pub particle_location_metadata: TfxScope,
    pub cubemap_volume: TfxScope,
    pub gear_plated_textures: TfxScope,
    pub gear_dye_0: TfxScope,
    pub gear_dye_1: TfxScope,
    pub gear_dye_2: TfxScope,
    pub gear_dye_decal: TfxScope,
    pub generic_array: TfxScope,
    pub gear_dye_skin: TfxScope,
    pub gear_dye_lips: TfxScope,
    pub gear_dye_hair: TfxScope,
    pub gear_dye_facial_layer_0_mask: TfxScope,
    pub gear_dye_facial_layer_0_material: TfxScope,
    pub gear_dye_facial_layer_1_mask: TfxScope,
    pub gear_dye_facial_layer_1_material: TfxScope,
    pub player_centered_cascaded_grid: TfxScope,
    pub gear_dye_012: TfxScope,
    pub color_grading_ubershader: TfxScope,
}

impl GlobalScopes {
    pub fn load(gctx: SharedGpuContext, globals: &SUnk808067a8) -> Self {
        let mut scopes = unsafe { MaybeUninit::<Self>::zeroed().assume_init() };

        let fields = scopes
            .fields()
            .map(|(name, _)| name.to_string())
            .collect_vec();

        for name in fields {
            let scope = globals.scopes.iter().find(|p| p.name.to_string() == name);

            match scope {
                Some(p) => {
                    let mut f = scopes.field_mut(&name).unwrap();

                    // cohae: We're using a pointer so the uninitialized value can't get dropped
                    let ptr = f.get_mut::<TfxScope>().unwrap() as *mut TfxScope;
                    unsafe {
                        ptr.write(
                            TfxScope::load(
                                package_manager()
                                    .read_tag_struct(p.scope)
                                    .unwrap_or_else(|_| panic!("Failed to read scope {name}")),
                                gctx.clone(),
                            )
                            .expect("Failed to load scope"),
                        );
                    }
                }
                None => {
                    panic!("Pipeline not found: {}", name)
                }
            }
        }

        scopes
    }
}

#[allow(non_snake_case)]
#[derive(FieldAccess)]
pub struct GlobalPipelines {
    // Shading
    pub clear_color_2_mrt: Box<Technique>,
    pub deferred_shading: Box<Technique>,
    pub deferred_shading_no_atm: Box<Technique>,
    pub global_lighting: Box<Technique>,
    pub global_lighting_ambient_only_and_shading: Box<Technique>,
    pub global_lighting_ambient_only: Box<Technique>,
    // pub global_lighting_and_shading_gel: Box<Technique>,
    pub global_lighting_and_shading: Box<Technique>,
    // pub global_lighting_gel: Box<Technique>,
    // pub global_lighting_masked_sun_and_shading_gel: Box<Technique>,
    // pub global_lighting_masked_sun_and_shading: Box<Technique>,
    // pub global_lighting_masked_sun_gel: Box<Technique>,
    // pub global_lighting_masked_sun: Box<Technique>,
    pub final_combine_no_film_curve: Box<Technique>,
    pub final_combine: Box<Technique>,

    // Post
    pub hdao: Box<Technique>,
    pub apply_ssao_to_light_buffers: Box<Technique>,
    pub ssao_bilateral_filter: Box<Technique>,
    pub ssao_compute_ao_3D_ps: Box<Technique>,

    pub fxaa: Box<Technique>,

    // Atmosphere/Sky
    pub hemisphere_sky_color_generate: Box<Technique>,
    pub sky: Box<Technique>,
    pub sky_lookup_generate_far: Box<Technique>,
    pub sky_lookup_generate_near: Box<Technique>,
    pub sky_hemisphere_copy_and_tint: Box<Technique>,
    pub sky_hemisphere_copy_frustum: Box<Technique>,
    pub sky_hemisphere_downsample_filter_ggx: Box<Technique>,
    pub sky_hemisphere_filter_cosine: Box<Technique>,
    pub sky_hemisphere_seed_inscattering: Box<Technique>,
    pub sky_hemisphere_spherical_blur: Box<Technique>,
    pub sky_direction_lookup_generate: Box<Technique>,
    pub sky_generate_sky_mask: Box<Technique>,

    pub cubemap_apply_cube_alpha_off_probes_off_relighting_off: Box<Technique>,
    pub cubemap_apply_cube_alpha_off_probes_off_relighting_on: Box<Technique>,
    pub cubemap_apply_cube_alpha_off_probes_on_relighting_off: Box<Technique>,
    pub cubemap_apply_cube_alpha_off_probes_on_relighting_on: Box<Technique>,
    pub cubemap_apply_cube_alpha_on_probes_off_relighting_off: Box<Technique>,
    pub cubemap_apply_cube_alpha_on_probes_off_relighting_on: Box<Technique>,
    pub cubemap_apply_cube_alpha_on_probes_on_relighting_off: Box<Technique>,
    pub cubemap_apply_cube_alpha_on_probes_on_relighting_on: Box<Technique>,
    pub cubemap_apply_cube_sphere_alpha_off_probes_off_relighting_off: Box<Technique>,
    pub cubemap_apply_cube_sphere_alpha_off_probes_off_relighting_on: Box<Technique>,
    pub cubemap_apply_cube_sphere_alpha_off_probes_on_relighting_off: Box<Technique>,
    pub cubemap_apply_cube_sphere_alpha_off_probes_on_relighting_on: Box<Technique>,
    pub cubemap_apply_cube_sphere_alpha_on_probes_off_relighting_off: Box<Technique>,
    pub cubemap_apply_cube_sphere_alpha_on_probes_off_relighting_on: Box<Technique>,
    pub cubemap_apply_cube_sphere_alpha_on_probes_on_relighting_off: Box<Technique>,
    pub cubemap_apply_cube_sphere_alpha_on_probes_on_relighting_on: Box<Technique>,
    pub cubemap_apply_sphere_alpha_off_probes_off_relighting_off: Box<Technique>,
    pub cubemap_apply_sphere_alpha_off_probes_off_relighting_on: Box<Technique>,
    pub cubemap_apply_sphere_alpha_off_probes_on_relighting_off: Box<Technique>,
    pub cubemap_apply_sphere_alpha_off_probes_on_relighting_on: Box<Technique>,
    pub cubemap_apply_sphere_alpha_on_probes_off_relighting_off: Box<Technique>,
    pub cubemap_apply_sphere_alpha_on_probes_off_relighting_on: Box<Technique>,
    pub cubemap_apply_sphere_alpha_on_probes_on_relighting_off: Box<Technique>,
    pub cubemap_apply_sphere_alpha_on_probes_on_relighting_on: Box<Technique>,
    pub cubemap_apply_parall_cube_alpha_off_probes_off_relighting_off: Box<Technique>,
    pub cubemap_apply_parall_cube_alpha_off_probes_off_relighting_on: Box<Technique>,
    pub cubemap_apply_parall_cube_alpha_off_probes_on_relighting_off: Box<Technique>,
    pub cubemap_apply_parall_cube_alpha_off_probes_on_relighting_on: Box<Technique>,
    pub cubemap_apply_parall_cube_alpha_on_probes_off_relighting_off: Box<Technique>,
    pub cubemap_apply_parall_cube_alpha_on_probes_off_relighting_on: Box<Technique>,
    pub cubemap_apply_parall_cube_alpha_on_probes_on_relighting_off: Box<Technique>,
    pub cubemap_apply_parall_cube_alpha_on_probes_on_relighting_on: Box<Technique>,
    pub cubemap_apply_parall_cube_sphere_alpha_off_probes_off_relighting_off: Box<Technique>,
    pub cubemap_apply_parall_cube_sphere_alpha_off_probes_off_relighting_on: Box<Technique>,
    pub cubemap_apply_parall_cube_sphere_alpha_off_probes_on_relighting_off: Box<Technique>,
    pub cubemap_apply_parall_cube_sphere_alpha_off_probes_on_relighting_on: Box<Technique>,
    pub cubemap_apply_parall_cube_sphere_alpha_on_probes_off_relighting_off: Box<Technique>,
    pub cubemap_apply_parall_cube_sphere_alpha_on_probes_off_relighting_on: Box<Technique>,
    pub cubemap_apply_parall_cube_sphere_alpha_on_probes_on_relighting_off: Box<Technique>,
    pub cubemap_apply_parall_cube_sphere_alpha_on_probes_on_relighting_on: Box<Technique>,
    pub cubemap_apply_parall_sphere_alpha_off_probes_off_relighting_off: Box<Technique>,
    pub cubemap_apply_parall_sphere_alpha_off_probes_off_relighting_on: Box<Technique>,
    pub cubemap_apply_parall_sphere_alpha_off_probes_on_relighting_off: Box<Technique>,
    pub cubemap_apply_parall_sphere_alpha_off_probes_on_relighting_on: Box<Technique>,
    pub cubemap_apply_parall_sphere_alpha_on_probes_off_relighting_off: Box<Technique>,
    pub cubemap_apply_parall_sphere_alpha_on_probes_off_relighting_on: Box<Technique>,
    pub cubemap_apply_parall_sphere_alpha_on_probes_on_relighting_off: Box<Technique>,
    pub cubemap_apply_parall_sphere_alpha_on_probes_on_relighting_on: Box<Technique>,

    pub cubemap_apply_override_hdr_with_sky_mask: Box<Technique>,
    pub cubemap_apply_override_opaque: Box<Technique>,
    pub cubemap_apply_override_with_sky_mask: Box<Technique>,
    pub cubemap_apply_sky_ambient_occlusion: Box<Technique>,
    pub cubemap_apply_sky_copy_ao_with_skybox_fade: Box<Technique>,
    pub cubemap_apply_sky_copy_ao: Box<Technique>,
    pub cubemap_apply_sky_specular_occlusion: Box<Technique>,
    pub cubemap_apply: Box<Technique>,

    // Debug
    pub debug_ambient_occlusion_source_color: Box<Technique>,
    pub debug_ambient_occlusion: Box<Technique>,
    pub debug_color_per_ao_status: Box<Technique>,
    pub debug_color_per_draw_call: Box<Technique>,
    pub debug_color_per_instance: Box<Technique>,
    pub debug_colored_overcoat_id: Box<Technique>,
    pub debug_colored_overcoat: Box<Technique>,
    pub debug_depth_edges: Box<Technique>,
    pub debug_depth_gradient: Box<Technique>,
    pub debug_depth_walkable: Box<Technique>,
    pub debug_depth: Box<Technique>,
    pub debug_diffuse_color: Box<Technique>,
    pub debug_diffuse_ibl: Box<Technique>,
    pub debug_diffuse_light: Box<Technique>,
    pub debug_diffuse_only: Box<Technique>,
    pub debug_emissive_intensity: Box<Technique>,
    pub debug_emissive_luminance: Box<Technique>,
    pub debug_emissive: Box<Technique>,
    pub debug_force_pbr_valid: Box<Technique>,
    pub debug_gbuffer_validation: Box<Technique>,
    pub debug_grey_diffuse: Box<Technique>,
    pub debug_local_ambient_occlusion: Box<Technique>,
    pub debug_metalness: Box<Technique>,
    pub debug_normal_compression: Box<Technique>,
    pub debug_normal_edges: Box<Technique>,
    pub debug_shader_chunked: Box<Technique>,
    pub debug_shader_cloth_skinned: Box<Technique>,
    pub debug_shader_cloth: Box<Technique>,
    pub debug_shader_decal: Box<Technique>,
    pub debug_shader_decorator: Box<Technique>,
    pub debug_shader_dq_skinned: Box<Technique>,
    pub debug_shader_lb_skinned: Box<Technique>,
    pub debug_shader_per_bone_scaled_lb_skinned: Box<Technique>,
    pub debug_shader_rigid_model: Box<Technique>,
    pub debug_shader_road_decal: Box<Technique>,
    pub debug_shader_terrain_thumbnail: Box<Technique>,
    pub debug_shader_terrain: Box<Technique>,
    pub debug_source_color_luminance: Box<Technique>,
    pub debug_source_color: Box<Technique>,
    pub debug_specular_color: Box<Technique>,
    pub debug_specular_ibl: Box<Technique>,
    pub debug_specular_light: Box<Technique>,
    pub debug_specular_occlusion: Box<Technique>,
    pub debug_specular_only: Box<Technique>,
    pub debug_specular_smoothness: Box<Technique>,
    pub debug_specular_tint: Box<Technique>,
    pub debug_texture_ao: Box<Technique>,
    pub debug_transmission: Box<Technique>,
    pub debug_valid_smoothness_heatmap: Box<Technique>,
    pub debug_valid_source_color_brightness: Box<Technique>,
    pub debug_valid_source_color_saturation: Box<Technique>,
    pub debug_vertex_color: Box<Technique>,
    pub debug_world_normal: Box<Technique>,

    // Feature renderer debug
    pub debug_cubemap_diffuse_opacity: Box<Technique>,
    pub debug_cubemap_diffuse_probes_alpha: Box<Technique>,
    pub debug_cubemap_diffuse_probes: Box<Technique>,
    pub debug_cubemap_overdraw: Box<Technique>,
    pub debug_cubemap_specular_opacity: Box<Technique>,
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
    pub fn load(gctx: SharedGpuContext, globals: &SUnk808067a8) -> Self {
        let mut pipelines = unsafe { MaybeUninit::<Self>::zeroed().assume_init() };

        let fields = pipelines
            .fields()
            .map(|(name, _)| name.to_string())
            .collect_vec();

        for name in fields {
            let pipeline = globals.unk20.iter().find(|p| p.name.to_string() == name);

            match pipeline {
                Some(p) => {
                    let mut f = pipelines.field_mut(&name).unwrap();

                    // cohae: We're using a pointer so the uninitialized value can't get dropped
                    let ptr = f.get_mut::<Box<Technique>>().unwrap() as *mut Box<Technique>;
                    let technique = Box::new(
                        load_technique(gctx.clone(), p.technique)
                            .map_err(|e| e.with_d3d_error(&gctx))
                            .expect("Failed to load global pipeline technique"),
                    );

                    // println!("Technique {} ({})", p.name.to_string(), p.technique);
                    // for (_rs, tstage) in technique.all_stages() {
                    //     if let Some(stage) = tstage {
                    //         if let Some(int) = &stage.bytecode {
                    //             if let Ok(d) = TfxBytecodeDecompiler::decompile(
                    //                 &int.opcodes,
                    //                 &stage.shader.bytecode_constants,
                    //             ) {
                    //                 println!("Expressions for stage {:?}:", stage.stage);
                    //                 println!("{}", d.pretty_print());
                    //             }
                    //         }
                    //     }
                    // }

                    unsafe {
                        ptr.write(technique);
                    }
                }
                None => {
                    panic!("Pipeline not found: {}", name)
                }
            }
        }

        pipelines
    }

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
            RenderDebugView::Emissive => &self.debug_emissive,
            RenderDebugView::EmissiveIntensity => &self.debug_emissive_intensity,
            RenderDebugView::EmissiveLuminance => &self.debug_emissive_luminance,
            RenderDebugView::GreyDiffuse => &self.debug_grey_diffuse,
            RenderDebugView::Depth => &self.debug_depth,
            RenderDebugView::DepthEdges => &self.debug_depth_edges,
            RenderDebugView::DepthGradient => &self.debug_depth_gradient,
            RenderDebugView::DepthWalkable => &self.debug_depth_walkable,
        }
    }
}
