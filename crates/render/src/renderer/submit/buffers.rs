use anyhow::Context;
use d3d11::dxgi;
use parking_lot::Mutex;

use crate::{
    gpu::command_list::CommandList,
    renderer::surface::{
        SizeRelativity, SurfaceDesc, SurfaceHandle, SurfaceProxy, SurfaceScale, Surfaces,
    },
    Gpu, Renderer,
};

pub struct Gbuffers {
    pub albedo: SurfaceHandle,
    pub normal: SurfaceHandle,
    pub normal_read: SurfaceHandle,
    pub third: SurfaceHandle,
    pub fourth: SurfaceHandle,

    pub depth: SurfaceHandle,
    pub depth_half: SurfaceHandle,
    pub depth_proxy: Mutex<SurfaceProxy>,
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
        let normal_read = surfaces.create_surface(base_resolution, desc_normal)?;
        let third = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("gbuffer_third", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R8g8b8a8Typeless)
                .view_format(dxgi::Format::R8g8b8a8Unorm)
                .build(),
        )?;
        let fourth = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("gbuffer_fourth", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R16g16b16a16Typeless)
                .view_format(dxgi::Format::R16g16b16a16Unorm)
                .build(),
        )?;
        // let depth = DepthState::create(gpu, base_resolution, "gbuffer_depth")?;
        let depth = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("gbuffer_depth", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R32g8x24Typeless)
                .view_format(dxgi::Format::D32FloatS8x24Uint)
                .build(),
        )?;
        let depth_half = surfaces.create_surface(
            base_resolution,
            SurfaceDesc::builder("gbuffer_depth_half", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R32g8x24Typeless)
                .view_format(dxgi::Format::D32FloatS8x24Uint)
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
            fourth,
            depth,
            depth_half,
            depth_proxy: Mutex::new(
                SurfaceProxy::new(
                    gpu,
                    surfaces.get(depth),
                    Some(dxgi::Format::R32FloatX8x24Typeless),
                    // Some(dxgi::Format::R32Float),
                    false,
                )
                .context("Failed to create surface proxy")?,
            ),
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
            &[self.albedo, self.normal, self.third, self.fourth],
            Some(self.depth),
        );
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
        surfaces
            .get(self.fourth)
            .clear_color(context, [0., 0., 0., 0.]);
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

        let [volumetrics_rt0, volumetrics_rt1, volumetrics_rt2, volumetrics_rt3] =
            std::array::from_fn(|i| {
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

// pub struct WaterBuffers {
//     pub water_uv: SurfaceHandle,
//     pub water_uv_healed: SurfaceHandle,
//     pub water_depth: SurfaceHandle,

//     pub water_reflection: SurfaceHandle,
//     pub water_reflection_healed: SurfaceHandle,
// }

// impl WaterBuffers {
//     pub fn create(surfaces: &Surfaces, base_resolution: (u32, u32)) -> anyhow::Result<Self> {
//         let water_uv = surfaces.create_surface(
//             base_resolution,
//             SurfaceDesc::builder("water_uv", SizeRelativity::RelativeToFramebuffer)
//                 .format(dxgi::Format::R16g16b16a16Typeless)
//                 .view_format(dxgi::Format::R16g16b16a16Unorm)
//                 .scale(SurfaceScale::Eighth)
//                 .build(),
//         )?;

//         let water_uv_healed = surfaces.create_surface(
//             base_resolution,
//             SurfaceDesc::builder("water_uv_healed", SizeRelativity::RelativeToFramebuffer)
//                 .format(dxgi::Format::R16g16b16a16Typeless)
//                 .view_format(dxgi::Format::R16g16b16a16Unorm)
//                 .scale(SurfaceScale::Eighth)
//                 .build(),
//         )?;

//         let water_depth = surfaces.create_surface(
//             base_resolution,
//             SurfaceDesc::builder("water_depth", SizeRelativity::RelativeToFramebuffer)
//                 .format(dxgi::Format::R32g8x24Typeless)
//                 .view_format(dxgi::Format::D32FloatS8x24Uint)
//                 .scale(SurfaceScale::Eighth)
//                 .build(),
//         )?;

//         let water_reflection = surfaces.create_surface(
//             base_resolution,
//             SurfaceDesc::builder("water_reflection", SizeRelativity::RelativeToFramebuffer)
//                 .format(dxgi::Format::R16g16b16a16Typeless)
//                 .view_format(dxgi::Format::R16g16b16a16Float)
//                 .scale(SurfaceScale::Quarter)
//                 .build(),
//         )?;

//         let water_reflection_healed = surfaces.create_surface(
//             base_resolution,
//             SurfaceDesc::builder(
//                 "water_reflection_healed",
//                 SizeRelativity::RelativeToFramebuffer,
//             )
//             .format(dxgi::Format::R16g16b16a16Typeless)
//             .view_format(dxgi::Format::R16g16b16a16Float)
//             .scale(SurfaceScale::Quarter)
//             .build(),
//         )?;

//         Ok(Self {
//             water_uv,
//             water_uv_healed,
//             water_depth,
//             water_reflection,
//             water_reflection_healed,
//         })
//     }
// }
