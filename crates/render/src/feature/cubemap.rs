use std::sync::Arc;

use alkahest_data::tfx::{
    PrimitiveType, RenderStage, ShaderStage,
    common::AxisAlignedBBox,
    features::{cubemap::SCubemapComponent, dynamic::RenderStageSubscription},
};
use d3d11::dxgi;
use glam::{Mat4, Quat, Vec3, Vec4, Vec4Swizzles, vec4};
use itertools::Itertools;

use super::FeatureRenderer;
use crate::{
    Gpu, Renderer,
    asset::{Handle, texture::Texture},
    gpu::{cbuffer::ConstantBuffer, command_list::CommandList},
    tfx::packet::CompactTransform,
    util::geometry,
};

pub struct CubemapRenderer {
    vb: d3d11::Buffer,
    ib: d3d11::Buffer,

    cb_vs: ConstantBuffer<CubemapTransform>,
    cb_ps: ConstantBuffer<CubemapPixelConstants>,

    texture_cubemap_specular: Handle<Texture>,
    texture_cubemap_alpha: Handle<Texture>,
    texture_voxel_diffuse: Handle<Texture>,

    cubemap: SCubemapComponent,
    bounds: AxisAlignedBBox,
}

impl CubemapRenderer {
    pub fn load(gpu: &Arc<Gpu>, cubemap: &SCubemapComponent) -> anyhow::Result<Self> {
        let vb = gpu.create_buffer(
            &d3d11::BufferDesc::builder()
                .byte_width(std::mem::size_of_val(geometry::CUBE_VERTICES) as u32)
                .usage(d3d11::Usage::Immutable)
                .bind_flags(d3d11::BindFlags::VERTEX_BUFFER)
                .build(),
            Some(bytemuck::cast_slice(geometry::CUBE_VERTICES)),
        )?;

        let ib = gpu.create_buffer(
            &d3d11::BufferDesc::builder()
                .byte_width(std::mem::size_of_val(geometry::CUBE_INDICES) as u32)
                .usage(d3d11::Usage::Immutable)
                .bind_flags(d3d11::BindFlags::INDEX_BUFFER)
                .build(),
            Some(bytemuck::cast_slice(geometry::CUBE_INDICES)),
        )?;

        Ok(Self {
            vb,
            ib,
            cb_vs: ConstantBuffer::create(gpu, None)?,
            cb_ps: ConstantBuffer::create(gpu, None)?,
            texture_cubemap_specular: Renderer::instance()
                .asset_manager
                .load(cubemap.texture_cube_specular_ibl),
            texture_cubemap_alpha: Renderer::instance()
                .asset_manager
                .load(cubemap.texture_cube_alpha),
            texture_voxel_diffuse: Renderer::instance()
                .asset_manager
                .load(cubemap.texture_voxel_diffuse),
            cubemap: cubemap.clone(),
            bounds: AxisAlignedBBox::NONE,
        })
    }
}

#[profiling::all_functions]
impl FeatureRenderer for CubemapRenderer {
    fn visibility_test(&mut self, _camera: &crate::camera::Camera) -> bool {
        // camera.is_visible(&self.bounds)
        true
    }

    fn extract_and_prepare(
        &mut self,
        renderer: &crate::Renderer,
        extracted_data: &dyn std::any::Any,
    ) {
        // TODO(cohae): lights shouldnt need to extract permutations at all
        let (obj_local_to_world, _permutation) = extracted_data
            .downcast_ref::<(CompactTransform, usize)>()
            .expect("Invalid extracted data type")
            .clone();

        let local_to_world = obj_local_to_world.to_mat4();
        let rotation = local_to_world.to_scale_rotation_translation().1;
        let rotation_neg = -rotation;

        self.cb_vs
            .write(
                &renderer.gpu.context(),
                &CubemapTransform {
                    translation: obj_local_to_world.translation().extend(1.0),
                    rotation: rotation_neg,
                    scale: self.cubemap.volume_extents,
                },
            )
            .unwrap();

        let cubemap_local_to_world =
            local_to_world * Mat4::from_scale(self.cubemap.volume_extents.xyz());
        let points = geometry::CUBE_VERTICES
            .iter()
            .map(|&v| cubemap_local_to_world.project_point3(v))
            .collect_vec();
        self.bounds = AxisAlignedBBox::from_points(&points);

        // Need param_13 (param_21), param_14 (param_20), param_18 (param_25)
        // let param_9 = {
        //     let mut fvar4 = self.cubemap.probes_resolution[2] as f32;
        //     if 0.0 <= 1.0 - fvar4 {
        //         fvar4 = 1.0;
        //     }
        //     let mut fvar3 = self.cubemap.probes_resolution[1] as f32;
        //     if 0.0 <= 1.0 - fvar3 {
        //         fvar3 = 1.0;
        //     }
        //     let mut fvar6 = self.cubemap.probes_resolution[0] as f32;
        //     if 0.0 <= 1.0 - fvar6 {
        //         fvar6 = 1.0;
        //     }
        //     Vec4::new(1.0 / fvar6, 1.0 / fvar3, 1.0 / fvar4, self.cubemap.unk13c)
        // };

        // let param_12 = self.cubemap.use_probes() as u8 as f32;

        // let param_13 = {
        //     let v10_x_abs = self.cubemap.unk90.x.abs();
        //     let v10_y_abs = self.cubemap.unk90.y.abs();
        //     let v10_z_abs = self.cubemap.unk90.z.abs();
        //     let mut v = Vec4::ZERO;
        //     v.x = v10_x_abs + v10_x_abs + self.cubemap.unk80.x;
        //     v.y = v10_y_abs + v10_y_abs + self.cubemap.unk80.y;
        //     v.z = v10_z_abs + v10_z_abs + self.cubemap.unk80.z;
        //     v.w = self.cubemap.unk80.w;
        //     v
        // };

        // // let param_14 = self.cubemap.unk70;
        // let param_18 = self.cubemap.unk70;

        let mut fvar27 = 0.05;
        // if 0.05 - param_13.w < 0.0 {
        //     fvar27 = param_13.w;
        // }

        let fvar26 = 0.0001;
        // if 0.0001 - param_18 < 0.0 {
        //     fvar26 = param_18;
        // }

        let unk16 = {
            fvar27 = 2.0 / fvar27;
            Vec4::new(1.0 / fvar27, 1.0 / fvar26, -(1.0 - fvar26) / fvar26, 0.0)
        };

        // let param_11 = self.cubemap.unk170.with_w(self.cubemap.unk74);
        let param_11 = self.cubemap.unk160.with_w(1.0);
        let mut fvar27 = param_11.z;
        if 0.5 < fvar27 {
            fvar27 = ((fvar27 - 0.5) + (fvar27 - 0.5)).powf(4.0);
            fvar27 = fvar27 * 60.0 + 1.0;
        } else {
            fvar27 = fvar27 + fvar27;
        }
        let unk14 = param_11.with_z(fvar27);

        // TODO(cohae): cb11[12] doesn't seem to be used, so i can't verify this
        let mut unk9 = self.cubemap.unk110;
        unk9.w_axis = Vec4::W;

        const EPSILON: Vec4 = Vec4::splat(0.0001);
        let fade0_nonzero = self.cubemap.unk40_fade1.abs().cmpgt(EPSILON);
        let fade0 = Vec4::select(
            fade0_nonzero,
            Vec4::ONE / self.cubemap.unk40_fade1,
            Vec4::splat(1000.0),
        );
        let fade1_nonzero = self.cubemap.unk50_fade2.abs().cmpgt(EPSILON);
        let fade1 = Vec4::select(
            fade1_nonzero,
            Vec4::ONE / self.cubemap.unk50_fade2,
            Vec4::splat(1000.0),
        );

        let unk15 = {
            let f_var3 = if (0.0 < self.cubemap.unk60.x) || (0.0 < self.cubemap.unk60.y) {
                1.0
            } else {
                0.0
            };

            let f_var4 = if (0.0 < self.cubemap.unk60.x) || (0.0 < self.cubemap.unk60.z) {
                1.0
            } else {
                0.0
            };

            let f_var9 = if (0.0 < self.cubemap.unk60.y) || (0.0 < self.cubemap.unk60.z) {
                1.0
            } else {
                0.0
            };

            Vec3::new(f_var9, f_var4, f_var3).extend(self.cubemap.unk60.w)
        };

        let unk19;
        let unk20;
        {
            let ext = &*Renderer::instance().externs;
            let sun_intensity = ext.get_global_channel_by_name("sun_intensity").x; // sun_intensity
            let sun_light_direction = ext.get_global_channel_by_name("sun_light_direction"); // sun_light_direction
            let unk8f552b79 = ext.get_global_channel_by_id(0x8f552b79).x; // ?
            let sun_color = ext.get_global_channel_by_name("sun_color"); // sun_color
            let cubemap_bounce_scale = ext.get_global_channel_by_name("cubemap_bounce_scale").x; // cubemap_bounce_scale
            let cubemap_relighting_sky_intensity = ext
                .get_global_channel_by_name("cubemap_relighting_sky_intensity")
                .x; // cubemap_relighting_sky_intensity

            let sun_height = sun_light_direction.z.max(0.5);
            let unk_sun_scale = sun_height * unk8f552b79 * (sun_intensity / 10.0);

            let param_11 = self.cubemap.unk160;
            let param_17 = self.cubemap.unk170;

            let fvar6 = (unk_sun_scale * sun_color - 1.0) * param_17.x + 1.0;
            let global_cubemap_scale = Vec4::ONE;

            unk19 = (cubemap_bounce_scale * param_11.x * fvar6).with_w(0.0);
            unk20 = (cubemap_relighting_sky_intensity * global_cubemap_scale * param_17.y + fvar6)
                .with_w(0.0);
        }

        self.cb_ps
            .write(
                &renderer.gpu.context(),
                &CubemapPixelConstants {
                    fade0: fade0.with_w(0.00),
                    fade1: fade1.with_w(self.cubemap.unk70.max(0.0001)),
                    fade2: (fade0 - 1.0).with_w(self.cubemap.unk30),
                    fade3: (fade1 - 1.0).with_w(8.5),

                    unk4: Vec4::Z,
                    unk5: self.cubemap.unkb0,
                    unk9,

                    unk13: vec4(
                        self.cubemap.unkb0.x_axis.x,
                        self.cubemap.unkb0.y_axis.y,
                        self.cubemap.unkb0.z_axis.z,
                        0.0,
                    ),
                    unk14: Vec4::new(0.08333, 0.08333, 0.16667, 1.00),
                    unk15: unk14, // Vec4::new(25.00, 0.50, 0.00, 0.00),
                    // p13
                    unk16: unk15,
                    unk17: unk16, // Vec4::new(0.025, 10000.00, -9999.00, 0.00), // Seems universal
                    unk18: Vec4::new(1.0, 1.0, 1.0, 0.0),
                    // Relighting constants
                    unk19, //: Vec4::new(1.0, 1.0, 1.0, 0.00),
                    unk20, //: Vec4::new(0.0, 0.0, 0.0, 0.00),
                },
            )
            .unwrap();
    }

    fn submit(&self, cmd: &mut CommandList, stage: RenderStage) {
        if stage != RenderStage::Cubemaps {
            return;
        }

        let tech = Renderer::instance()
            .globals
            .pipelines
            .get_specialized_cubemap_pipeline(
                self.cubemap.shape(),
                self.cubemap.use_alpha(),
                self.cubemap.use_probes(),
                // TODO(cohae): Relighting blows out a lot of cubemaps
                self.cubemap.use_relighting(),
                false,
                // self.cubemap.use_parallax(),
            );

        self.cb_vs.bind(cmd, ShaderStage::Vertex, 11);
        self.cb_ps.bind(cmd, ShaderStage::Pixel, 11);

        tech.bind(cmd).unwrap();

        cmd.set_input_topology(PrimitiveType::Triangles);
        cmd.set_input_layout(1); // float3 v0 : POSITION0, // Format DXGI_FORMAT_R32G32B32_FLOAT size 12

        cmd.input_assembler_set_index_buffer(&self.ib, dxgi::Format::R16Uint, 0);
        cmd.input_assembler_set_vertex_buffers(0, &[Some(&self.vb)], Some(&[12]), Some(&[0]))
            .unwrap();

        cmd.pixel_set_shader_resources(1, &[None]); // Unbind t1
        self.texture_cubemap_specular
            .get()
            .inspect(|t| t.bind(cmd, 0, alkahest_data::tfx::ShaderStage::Pixel));
        self.texture_cubemap_alpha
            .get()
            .inspect(|t| t.bind(cmd, 1, alkahest_data::tfx::ShaderStage::Pixel));
        self.texture_voxel_diffuse
            .get()
            .inspect(|t| t.bind(cmd, 2, alkahest_data::tfx::ShaderStage::Pixel));

        cmd.draw_indexed(geometry::CUBE_INDICES.len() as u32, 0, 0);
        cmd.flush_states();
    }

    fn subscribed_stages(&self) -> RenderStageSubscription {
        RenderStageSubscription::CUBEMAPS
    }
}

#[repr(C)]
pub struct CubemapTransform {
    pub translation: Vec4,
    pub rotation: Quat,
    pub scale: Vec4,
}

#[repr(C)]
pub struct CubemapPixelConstants {
    // Fade constants/points. W component is used for miscellaneous data
    pub fade0: Vec4, // cb11[0]
    pub fade1: Vec4, // cb11[1]
    pub fade2: Vec4, // cb11[2]
    pub fade3: Vec4, // cb11[3]
    pub unk4: Vec4,  // cb11[4]
    pub unk5: Mat4,  // cb11[5..=8]
    pub unk9: Mat4,  // cb11[9..=12]
    pub unk13: Vec4,
    pub unk14: Vec4, // cb11[14]
    pub unk15: Vec4, // cb11[15]
    pub unk16: Vec4, // cb11[16]
    pub unk17: Vec4, // cb11[17]
    pub unk18: Vec4, // cb11[18]
    pub unk19: Vec4, // cb11[19]
    pub unk20: Vec4, // cb11[20]
}
