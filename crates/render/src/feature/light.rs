use alkahest_data::tfx::{
    common::AxisAlignedBBox,
    features::{
        dynamic::RenderStageSubscription,
        light::{SLight, SShadowingLight},
    },
    PrimitiveType, RenderStage,
};
use d3d11::dxgi;
use glam::{Mat4, Vec3, Vec4, Vec4Swizzles};
use itertools::Itertools;
use tiger_pkg::TagHash;

use super::FeatureRenderer;
use crate::{
    camera::Camera,
    tfx::{
        externs::{self, DeferredLight, SimpleGeometry},
        packet::CompactTransform,
        technique::Technique,
    },
    util::geometry,
    Renderer,
};
pub struct LightRenderer {
    technique_lighting_apply: Technique,
    technique_volumetrics: Option<Technique>,
    // technique_light_probe_apply: Technique,

    // TODO(cohae): This should be a shared resource (eg. a struct in the renderer that we can use instead of recreating it for every light/cubemap)
    vb: d3d11::Buffer,
    ib: d3d11::Buffer,

    local_to_world: glam::Mat4,
    light_space_transform: glam::Mat4,
    bounds: Option<AxisAlignedBBox>,
}

impl LightRenderer {
    pub fn new(
        renderer: &Renderer,
        light: &SLight,
        bounds: AxisAlignedBBox,
    ) -> anyhow::Result<Box<Self>> {
        Self::new_impl(
            renderer,
            light.technique_lighting_apply,
            light.technique_volumetrics,
            // light.technique_light_probe_apply,
            light.light_space_transform,
            Some(bounds),
        )
    }

    pub fn new_shadowing(
        renderer: &Renderer,
        light: &SShadowingLight,
    ) -> anyhow::Result<Box<Self>> {
        Self::new_impl(
            renderer,
            light.technique_lighting_apply,
            light.technique_volumetrics,
            light.light_space_transform,
            None,
        )
    }

    fn new_impl(
        renderer: &Renderer,
        technique_shading: TagHash,
        technique_volumetrics: TagHash,
        // technique_light_probe: TagHash,
        light_space_transform: Mat4,
        bounds: Option<AxisAlignedBBox>,
    ) -> anyhow::Result<Box<Self>> {
        let vb = renderer.gpu.create_buffer(
            &d3d11::BufferDesc::builder()
                .byte_width(std::mem::size_of_val(geometry::CUBE_VERTICES) as u32)
                .usage(d3d11::Usage::Immutable)
                .bind_flags(d3d11::BindFlags::VERTEX_BUFFER)
                .build(),
            Some(bytemuck::cast_slice(geometry::CUBE_VERTICES)),
        )?;

        let ib = renderer.gpu.create_buffer(
            &d3d11::BufferDesc::builder()
                .byte_width(std::mem::size_of_val(geometry::CUBE_INDICES) as u32)
                .usage(d3d11::Usage::Immutable)
                .bind_flags(d3d11::BindFlags::INDEX_BUFFER)
                .build(),
            Some(bytemuck::cast_slice(geometry::CUBE_INDICES)),
        )?;

        Ok(Box::new(Self {
            technique_lighting_apply: Technique::load(&renderer.gpu, technique_shading)?,
            technique_volumetrics: technique_volumetrics
                .is_some()
                .then(|| Technique::load(&renderer.gpu, technique_volumetrics))
                .transpose()?,
            // technique_light_probe_apply: Technique::load(&renderer.gpu, technique_light_probe)?,
            vb,
            ib,
            local_to_world: Mat4::IDENTITY,
            light_space_transform,
            bounds,
        }))
    }
}

impl FeatureRenderer for LightRenderer {
    fn visibility_test(&mut self, camera: &Camera) -> bool {
        if let Some(ref bounds) = self.bounds {
            camera.is_visible(bounds)
        } else {
            true
        }
    }

    fn extract_and_prepare(
        &mut self,
        _renderer: &crate::Renderer,
        extracted_data: &dyn std::any::Any,
    ) {
        // TODO(cohae): lights shouldnt need to extract permutations at all
        let (obj_local_to_world, _permutation) = extracted_data
            .downcast_ref::<(CompactTransform, usize)>()
            .expect("Invalid extracted data type")
            .clone();

        self.local_to_world = obj_local_to_world.to_mat4();

        let local_to_world_scaled = self.local_to_world * self.light_space_transform;
        let points = geometry::CUBE_VERTICES
            .iter()
            .map(|&v| local_to_world_scaled.project_point3(v))
            .collect_vec();

        self.bounds = Some(AxisAlignedBBox::from_points(&points));
    }

    fn submit(&self, cmd: &mut crate::gpu::command_list::CommandList, stage: RenderStage) {
        if stage != RenderStage::LightingApply {
            // TODO
            return;
        }

        {
            // let (scale, _rotation, _translation) =
            //     self.local_to_world.to_scale_rotation_translation();

            let local_to_world_scaled = self.local_to_world * self.light_space_transform;
            let externs = Renderer::instance().externs.get();
            cmd.externs.simple_geometry = Some(Box::new(SimpleGeometry {
                local_to_world: externs.view.world_to_projective
                    * local_to_world_scaled
                    * Mat4::from_scale(Vec3::NEG_ONE),
            }));

            let view_translation_inverse_mat4 = Mat4::from_translation(-externs.view.position());
            let local_to_world_relative = view_translation_inverse_mat4 * self.local_to_world;

            let (min, max) = compute_light_bounds(self.light_space_transform);
            let light_local_to_world = compute_light_local_to_world(self.local_to_world, min, max);

            cmd.externs.deferred_light = Some(Box::new(DeferredLight {
                // unk40: local_to_world_relative.inverse().transpose(),
                unk40: (view_translation_inverse_mat4 * light_local_to_world).inverse(),
                unk80: local_to_world_relative,

                ..Default::default()
            }));

            cmd.externs.rigid_model = Some(Box::new(externs::RigidModel {
                local_to_world: light_local_to_world,
                ..Default::default()
            }));
        }

        self.technique_lighting_apply.bind(cmd).unwrap();

        cmd.set_input_topology(PrimitiveType::Triangles);
        cmd.set_input_layout(1); // float3 v0 : POSITION0, // Format DXGI_FORMAT_R32G32B32_FLOAT size 12

        cmd.input_assembler_set_index_buffer(&self.ib, dxgi::Format::R16Uint, 0);
        cmd.input_assembler_set_vertex_buffers(0, &[Some(&self.vb)], Some(&[12]), Some(&[0]))
            .unwrap();

        cmd.draw_indexed(geometry::CUBE_INDICES.len() as u32, 0, 0);
        cmd.flush_states();
    }

    fn subscribed_stages(&self) -> RenderStageSubscription {
        RenderStageSubscription::LIGHTING_APPLY
            | RenderStageSubscription::LIGHT_PROBE_APPLY
            | RenderStageSubscription::VOLUMETRICS
    }
}

fn compute_light_bounds(light_space_transform: Mat4) -> (Vec3, Vec3) {
    let mut points = [
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(-1.0, 1.0, 1.0),
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(1.0, 1.0, -1.0),
        Vec3::new(1.0, 1.0, 1.0),
    ];

    for point in &mut points {
        let p = light_space_transform.mul_vec4(point.extend(1.0));
        let point_w_abs = (-p.wwww()).abs();
        *point = Vec4::select(
            point_w_abs.cmpge(Vec4::splat(0.0001)),
            p / p.wwww(),
            Vec4::W,
        )
        .truncate();
    }

    points
        .iter()
        .fold((Vec3::MAX, Vec3::MIN), |(min, max), &point| {
            (min.min(point), max.max(point))
        })
}

fn compute_light_local_to_world(node_local_to_world: Mat4, min: Vec3, max: Vec3) -> Mat4 {
    let bounds_center = min.midpoint(max);
    let bounds_half_extents = (max - min) / 2.0;

    // First matrix operation ("mat"):
    // Each column is computed by scaling one of node_local_to_world’s axes by the corresponding component of bounds_half_extents,
    // except for the w-axis which is a linear combination of the x, y, and z axes plus the original w-axis.
    let mat = Mat4 {
        x_axis: node_local_to_world.x_axis * bounds_half_extents.x,
        y_axis: node_local_to_world.y_axis * bounds_half_extents.y,
        z_axis: node_local_to_world.z_axis * bounds_half_extents.z,
        w_axis: node_local_to_world.x_axis * bounds_center.x
            + node_local_to_world.y_axis * bounds_center.y
            + node_local_to_world.z_axis * bounds_center.z
            + node_local_to_world.w_axis,
    };

    // Second matrix operation ("mat_scaled"):
    // Scale the x, y, and z axes by 2, and subtract all three from the w-axis.
    let mat_scaled = Mat4 {
        x_axis: mat.x_axis * 2.0,
        y_axis: mat.y_axis * 2.0,
        z_axis: mat.z_axis * 2.0,
        w_axis: mat.w_axis - mat.x_axis - mat.y_axis - mat.z_axis,
    };

    // Third matrix operation (computing light_local_to_world):
    // Rearrange the columns of mat_scaled: swap the x and z axes, leaving y and w unchanged.

    Mat4 {
        x_axis: mat_scaled.z_axis,
        y_axis: mat_scaled.y_axis,
        z_axis: mat_scaled.x_axis,
        w_axis: mat_scaled.w_axis,
    }
}
