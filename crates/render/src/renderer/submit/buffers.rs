use anyhow::Context;
use d3d11::dxgi;
use parking_lot::Mutex;

use crate::{
    Gpu, Renderer,
    gpu::command_list::CommandList,
    renderer::surface::{
        SizeRelativity, SurfaceDesc, SurfaceHandle, SurfaceProxy, SurfaceScale, Surfaces,
    },
};

pub struct Gbuffers {
    pub albedo: SurfaceHandle,
    pub normal: SurfaceHandle,
    pub normal_read: SurfaceHandle,
    pub third: SurfaceHandle,

    pub depth: SurfaceHandle,
    pub depth_half: SurfaceHandle,
    pub depth_proxy: Mutex<SurfaceProxy>,
    pub albedo_proxy: Mutex<SurfaceProxy>,
    pub third_proxy: Mutex<SurfaceProxy>,

    pub uber_depth_half: SurfaceHandle,
    pub uber_depth_quarter: SurfaceHandle,
    pub uber_depth_eighth: SurfaceHandle,
    // pub uber_depth_sixteenth: SurfaceHandle,
    // pub uber_depth_fortieth: SurfaceHandle,
}

impl Gbuffers {
    pub fn create(
        gpu: &Gpu,
        surfaces: &Surfaces,
        base_resolution: (u32, u32),
    ) -> anyhow::Result<Self> {
        let albedo = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("gbuffer_albedo", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R8g8b8a8Typeless)
                .view_format(dxgi::Format::R8g8b8a8UnormSrgb)
                .build(),
        )?;
        let desc_normal =
            SurfaceDesc::builder("gbuffer_normal", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R10g10b10a2Typeless)
                .view_format(dxgi::Format::R10g10b10a2Unorm)
                .build();
        let normal = surfaces.create_surface(base_resolution, desc_normal.clone())?;
        let normal_read = surfaces.create_surface(
            base_resolution,
            desc_normal.with_name("gbuffer_normal_read"),
        )?;
        let third = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("gbuffer_third", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R8g8b8a8Typeless)
                .view_format(dxgi::Format::R8g8b8a8Unorm)
                .build(),
        )?;
        // let depth = DepthState::create(gpu, base_resolution, "gbuffer_depth")?;
        let depth = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("gbuffer_depth", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R32g8x24Typeless)
                .depth_format(dxgi::Format::D32FloatS8x24Uint)
                .build(),
        )?;
        let depth_half = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("gbuffer_depth_half", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R32g8x24Typeless)
                .depth_format(dxgi::Format::D32FloatS8x24Uint)
                .scale(SurfaceScale::Half)
                .build(),
        )?;

        let uber_depth_half = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("uber_depth_half", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R16g16Typeless)
                .view_format(dxgi::Format::R16g16Float)
                .scale(SurfaceScale::Half)
                .create_uav(true)
                .build(),
        )?;

        let uber_depth_quarter = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("uber_depth_quarter", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R16g16b16a16Typeless)
                .view_format(dxgi::Format::R16g16b16a16Float)
                .scale(SurfaceScale::Quarter)
                .create_uav(true)
                .build(),
        )?;

        let uber_depth_eigth = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("uber_depth_eigth", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R16g16Typeless)
                .view_format(dxgi::Format::R16g16Float)
                .scale(SurfaceScale::Eighth)
                .create_uav(true)
                .build(),
        )?;

        Ok(Gbuffers {
            albedo,
            normal,
            normal_read,
            third,
            depth,
            depth_half,
            depth_proxy: Mutex::new(
                SurfaceProxy::new(
                    gpu,
                    surfaces.get(depth),
                    Some(dxgi::Format::R32FloatX8x24Typeless),
                    false,
                )
                .context("Failed to create surface proxy")?,
            ),
            albedo_proxy: Mutex::new(SurfaceProxy::new(gpu, surfaces.get(albedo), None, false)?),
            third_proxy: Mutex::new(SurfaceProxy::new(
                gpu,
                surfaces.get(third),
                Some(dxgi::Format::R8g8b8a8Unorm),
                // Some(dxgi::Format::R8g8b8a8Typeless),
                false,
            )?),

            uber_depth_half,
            uber_depth_quarter,
            uber_depth_eighth: uber_depth_eigth,
        })
    }

    pub fn bind(&self, cmd: &mut CommandList, renderer: &Renderer) {
        renderer.bind_surfaces(
            cmd,
            &[self.albedo, self.normal, self.third],
            Some(self.depth),
        );
    }

    pub fn bind_depth_only(&self, cmd: &mut CommandList, renderer: &Renderer) {
        renderer.bind_surfaces(cmd, &[], Some(self.depth));
    }

    pub fn clear(&self, context: &d3d11::DeviceContext, surfaces: &Surfaces) {
        surfaces
            .get(self.albedo)
            .clear_color(context, [0., 0., 0., 0.]);
        surfaces
            .get(self.normal)
            .clear_color(context, [0., 0., 0., 0.]);
        surfaces
            .get(self.third)
            .clear_color(context, [0., 0.5, 0., 0.]);
        surfaces.get(self.depth).clear_depth(context, 0., 0xff);
    }
}

pub struct LightBuffers {
    pub light_diffuse: SurfaceHandle,
    pub light_specular: SurfaceHandle,
    pub light_specular_ibl: SurfaceHandle,
    pub vertex_ao: SurfaceHandle,
    pub distortion: SurfaceHandle,

    pub volumetrics_rt0: SurfaceHandle,
    pub volumetrics_rt1: SurfaceHandle,
    pub volumetrics_rt2: SurfaceHandle,
    pub volumetrics_rt3: SurfaceHandle,

    pub volumetrics_upres: SurfaceHandle,

    pub ssao: SurfaceHandle,
    /// Intermediate buffer used for holding the horizontal blur result of the SSAO pass
    pub ssao_pong: SurfaceHandle,
}

impl LightBuffers {
    pub fn create(surfaces: &Surfaces, base_resolution: (u32, u32)) -> anyhow::Result<Self> {
        let light_diffuse = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("light_diffuse", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R11g11b10Float)
                .build(),
        )?;

        let light_specular = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("light_specular", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R11g11b10Float)
                .build(),
        )?;

        let light_specular_ibl = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("light_specular_ibl", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R11g11b10Float)
                .build(),
        )?;

        let gbuffer_ao = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("gbuffer_ao", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R8Typeless)
                .view_format(dxgi::Format::R8Unorm)
                .build(),
        )?;

        let distortion = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("distortion", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R8g8b8a8Typeless)
                .view_format(dxgi::Format::R8g8b8a8Unorm)
                .scale(SurfaceScale::Half)
                .build(),
        )?;

        let [
            volumetrics_rt0,
            volumetrics_rt1,
            volumetrics_rt2,
            volumetrics_rt3,
        ] = std::array::from_fn(|i| {
            surfaces.create_surface(
                base_resolution,
                SurfaceDesc::builder(
                    format!("volumetrics_{i}"),
                    SizeRelativity::RelativeToFramebuffer,
                )
                .format(dxgi::Format::R16g16b16a16Typeless)
                .view_format(dxgi::Format::R16g16b16a16Float)
                .scale(SurfaceScale::Eighth)
                .build(),
            )
        });

        let volumetrics_upres = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder(
                "volumetrics_upres".to_string(),
                SizeRelativity::RelativeToFramebuffer,
            )
            .format(dxgi::Format::R16g16b16a16Typeless)
            .view_format(dxgi::Format::R16g16b16a16Float)
            .scale(SurfaceScale::Half)
            .build(),
        )?;

        let ssao_desc = SurfaceDesc::builder("ssao", SizeRelativity::RelativeToFramebuffer)
            .format(dxgi::Format::R8g8Typeless)
            .view_format(dxgi::Format::R8g8Unorm)
            .build();

        let ssao = surfaces.create_surface(base_resolution, ssao_desc.clone())?;
        let ssao_pong = surfaces.create_surface(base_resolution, ssao_desc)?;

        Ok(Self {
            light_diffuse,
            light_specular,
            light_specular_ibl,
            vertex_ao: gbuffer_ao,
            distortion,
            volumetrics_rt0: volumetrics_rt0?,
            volumetrics_rt1: volumetrics_rt1?,
            volumetrics_rt2: volumetrics_rt2?,
            volumetrics_rt3: volumetrics_rt3?,
            volumetrics_upres,
            ssao,
            ssao_pong,
        })
    }

    pub fn clear(&self, context: &d3d11::DeviceContext, surfaces: &Surfaces) {
        surfaces
            .get(self.light_diffuse)
            .clear_color(context, [0., 0., 0., 1.]);
        surfaces
            .get(self.light_specular)
            .clear_color(context, [0., 0., 0., 1.]);
        surfaces
            .get(self.light_specular_ibl)
            .clear_color(context, [0., 0., 0., 1.]);

        for rt in &[
            self.volumetrics_rt0,
            self.volumetrics_rt1,
            self.volumetrics_rt2,
            self.volumetrics_rt3,
        ] {
            surfaces.get(*rt).clear_color(context, [0., 0., 0., 1.]);
        }
    }

    pub fn bind_ibl_vertex_ao(&self, context: &d3d11::DeviceContext, surfaces: &Surfaces) {
        context.rasterizer_set_viewports(&[surfaces.get(self.light_diffuse).viewport()]);
        context.output_merger_set_render_targets(
            &[
                Some(surfaces.get(self.light_diffuse).rtv.as_ref().unwrap()),
                Some(surfaces.get(self.light_specular_ibl).rtv.as_ref().unwrap()),
                Some(surfaces.get(self.vertex_ao).rtv.as_ref().unwrap()),
            ],
            None,
        );
    }

    pub fn bind_diffuse_ibl(&self, context: &d3d11::DeviceContext, surfaces: &Surfaces) {
        context.output_merger_set_render_targets(
            &[
                Some(surfaces.get(self.light_diffuse).rtv.as_ref().unwrap()),
                Some(surfaces.get(self.light_specular_ibl).rtv.as_ref().unwrap()),
            ],
            None,
        );
    }

    pub fn bind_diffuse_specular(&self, context: &d3d11::DeviceContext, surfaces: &Surfaces) {
        context.output_merger_set_render_targets(
            &[
                Some(surfaces.get(self.light_diffuse).rtv.as_ref().unwrap()),
                Some(surfaces.get(self.light_specular).rtv.as_ref().unwrap()),
            ],
            None,
        );
    }

    pub fn bind_volumetrics(&self, renderer: &Renderer, cmd: &mut CommandList) {
        renderer.bind_surfaces(
            cmd,
            &[
                self.volumetrics_rt0,
                self.volumetrics_rt1,
                self.volumetrics_rt2,
                self.volumetrics_rt3,
            ],
            None,
        );
    }
}

pub struct WaterBuffers {
    pub water_uv: SurfaceHandle,
    pub water_uv_healed: SurfaceHandle,
    pub water_depth: SurfaceHandle,

    pub water_reflection: SurfaceHandle,
    pub water_reflection_healed: SurfaceHandle,
}

impl WaterBuffers {
    pub fn create(surfaces: &Surfaces, base_resolution: (u32, u32)) -> anyhow::Result<Self> {
        let water_uv = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("water_uv", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R16g16b16a16Typeless)
                .view_format(dxgi::Format::R16g16b16a16Unorm)
                .scale(SurfaceScale::Eighth)
                .build(),
        )?;

        let water_uv_healed = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("water_uv_healed", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R16g16b16a16Typeless)
                .view_format(dxgi::Format::R16g16b16a16Unorm)
                .scale(SurfaceScale::Eighth)
                .build(),
        )?;

        let water_depth = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("water_depth", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R32g8x24Typeless)
                .depth_format(dxgi::Format::D32FloatS8x24Uint)
                .scale(SurfaceScale::Eighth)
                .build(),
        )?;

        let water_reflection = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("water_reflection", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R16g16b16a16Typeless)
                .view_format(dxgi::Format::R16g16b16a16Float)
                .scale(SurfaceScale::Quarter)
                .build(),
        )?;

        let water_reflection_healed = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder(
                "water_reflection_healed",
                SizeRelativity::RelativeToFramebuffer,
            )
            .format(dxgi::Format::R16g16b16a16Typeless)
            .view_format(dxgi::Format::R16g16b16a16Float)
            .scale(SurfaceScale::Quarter)
            .build(),
        )?;

        Ok(Self {
            water_uv,
            water_uv_healed,
            water_depth,
            water_reflection,
            water_reflection_healed,
        })
    }
}

pub struct BloomBuffers {
    pub bloom_3rd: SurfaceHandle,
    pub bloom_3rd_temp: SurfaceHandle,
    pub bloom_3rd_combined: SurfaceHandle,

    pub bloom_6th: SurfaceHandle,
    pub bloom_6th_temp: SurfaceHandle,
    pub bloom_6th_combined: SurfaceHandle,

    pub bloom_12th: SurfaceHandle,
    pub bloom_12th_temp: SurfaceHandle,
    pub bloom_12th_combined: SurfaceHandle,

    pub bloom_12th_half_width: SurfaceHandle,
    pub bloom_12th_quarter_width: SurfaceHandle,
    pub bloom_12th_quarter_width_temp: SurfaceHandle,

    pub bloom_24th: SurfaceHandle,
    pub bloom_24th_temp: SurfaceHandle,

    pub bloom_final: SurfaceHandle,

    pub autoexposure_sample_columns: SurfaceHandle,
    pub autoexposure_sample_columns_cpu: Mutex<SurfaceProxy>,
}

impl BloomBuffers {
    pub fn create(
        gpu: &Gpu,
        surfaces: &Surfaces,
        base_resolution: (u32, u32),
    ) -> anyhow::Result<Self> {
        let create_desc = |name: &str, scale: SurfaceScale| {
            let desc = SurfaceDesc::builder(name, SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R16g16b16a16Float)
                .scale(scale)
                .build();
            surfaces.create_surface(base_resolution, desc)
        };

        let bloom_3rd = create_desc("bloom_3rd", SurfaceScale::Third)?;
        let bloom_3rd_temp = create_desc("bloom_3rd_temp", SurfaceScale::Third)?;
        let bloom_3rd_combined = create_desc("bloom_3rd_combined", SurfaceScale::Third)?;

        let bloom_6th = create_desc("bloom_6th", SurfaceScale::Sixth)?;
        let bloom_6th_temp = create_desc("bloom_6th_temp", SurfaceScale::Sixth)?;
        let bloom_6th_combined = create_desc("bloom_6th_combined", SurfaceScale::Sixth)?;

        let bloom_12th = create_desc("bloom_12th", SurfaceScale::Twelfth)?;
        let bloom_12th_temp = create_desc("bloom_12th_temp", SurfaceScale::Twelfth)?;
        let bloom_12th_combined = create_desc("bloom_12th_combined", SurfaceScale::Twelfth)?;

        let bloom_12th_half_width =
            create_desc("bloom_12th_half_width", SurfaceScale::Nth(12.0 * 2.0, 12.0))?;
        let bloom_12th_quarter_width = create_desc(
            "bloom_12th_quarter_width",
            SurfaceScale::Nth(12.0 * 4.0, 12.0),
        )?;
        let bloom_12th_quarter_width_temp = create_desc(
            "bloom_12th_quarter_width_temp",
            SurfaceScale::Nth(12.0 * 4.0, 12.0),
        )?;

        let bloom_24th = create_desc("bloom_24th", SurfaceScale::TwentyFourth)?;
        let bloom_24th_temp = create_desc("bloom_24th_temp", SurfaceScale::TwentyFourth)?;

        let bloom_final = create_desc("bloom_final", SurfaceScale::Half)?;

        let autoexposure_sample_columns = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder(
                "autoexposure_sample_columns",
                SizeRelativity::RelativeToFramebuffer,
            )
            .format(dxgi::Format::R32g32b32a32Float)
            .view_format(dxgi::Format::R32g32b32a32Float)
            .scale(SurfaceScale::Nth(48.0, 0.0))
            .build(),
        )?;

        let autoexposure_sample_columns_cpu =
            SurfaceProxy::new(gpu, surfaces.get(autoexposure_sample_columns), None, true)
                .expect("Failed to create CPU autoexposure_sample_columns surface proxy");

        Ok(Self {
            bloom_3rd,
            bloom_3rd_temp,
            bloom_3rd_combined,
            bloom_6th,
            bloom_6th_temp,
            bloom_6th_combined,
            bloom_12th,
            bloom_12th_temp,
            bloom_12th_combined,
            bloom_12th_half_width,
            bloom_12th_quarter_width,
            bloom_12th_quarter_width_temp,
            bloom_24th,
            bloom_24th_temp,
            bloom_final,

            autoexposure_sample_columns,
            autoexposure_sample_columns_cpu: Mutex::new(autoexposure_sample_columns_cpu),
        })
    }

    pub fn clear_results(&self, cmd: &mut CommandList) {
        Renderer::instance().clear_surface(cmd, self.bloom_final, [0.0, 0.0, 0.0, 1.0]);
    }
}

pub struct AtmosphereBuffers {
    pub sky_mask_initial: SurfaceHandle,
    pub sky_mask: SurfaceHandle,
    pub sky_mask_temp: SurfaceHandle,

    pub sky_lookup_near: SurfaceHandle,
    pub sky_lookup_far: SurfaceHandle,
}

impl AtmosphereBuffers {
    pub fn create(surfaces: &Surfaces, base_resolution: (u32, u32)) -> anyhow::Result<Self> {
        let sky_mask_initial = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("sky_mask_initial", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R8Typeless)
                .view_format(dxgi::Format::R8Unorm)
                .scale(SurfaceScale::Quarter)
                .build(),
        )?;

        let sky_mask = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("sky_mask", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R8Typeless)
                .view_format(dxgi::Format::R8Unorm)
                .scale(SurfaceScale::Eighth)
                .build(),
        )?;

        let sky_mask_temp = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("sky_mask_temp", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R8Typeless)
                .view_format(dxgi::Format::R8Unorm)
                .scale(SurfaceScale::Eighth)
                .build(),
        )?;

        let sky_lookup_desc = SurfaceDesc::builder("sky_lookup", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R16g16b16a16Typeless)
                .view_format(dxgi::Format::R16g16b16a16Float)
                .scale(SurfaceScale::even(1.0 / 0.375)) // 3 x (1/8) resolution
                .build();

        let sky_lookup_near = surfaces.create_surface(
            base_resolution,
            sky_lookup_desc.clone().with_name("sky_lookup_near"),
        )?;

        let sky_lookup_far = surfaces
            .create_surface(base_resolution, sky_lookup_desc.with_name("sky_lookup_far"))?;

        Ok(Self {
            sky_mask_initial,
            sky_mask,
            sky_mask_temp,
            sky_lookup_near,
            sky_lookup_far,
        })
    }
}
