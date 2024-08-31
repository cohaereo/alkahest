use alkahest_data::occlusion::Aabb;
use bevy_ecs::{
    query::{Or, QueryData, QueryFilter, With},
    system::{In, Query},
};
use glam::{Mat4, Vec3};

use super::{
    render::{
        dynamic_geometry::DynamicModelComponent, light::ShadowMapRenderer,
        static_geometry::StaticModelSingle,
    },
    tags::NodeFilter,
    transform::Transform,
    visibility::ViewVisibility,
};
use crate::{ecs::visibility::VisibilityHelper, gpu_event, renderer::RendererShared, Color};

#[derive(Debug, Copy, Clone)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}

impl Sphere {
    pub fn transform(&self, transform: Mat4) -> Self {
        Self {
            center: transform.transform_point3(self.center),
            radius: self.radius * transform.to_scale_rotation_translation().0.max_element(),
        }
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub struct Plane {
    pub direction: Vec3,
    pub d: f32,
}

impl Plane {
    pub fn new(x: f32, y: f32, z: f32, d: f32) -> Self {
        Self {
            direction: Vec3::new(x, y, z),
            d,
        }
    }

    pub fn normalize(mut self) -> Self {
        let len = self.direction.length();

        self.direction /= len;
        self.d /= len;

        self
    }

    pub fn distance(self, point: Vec3) -> f32 {
        self.direction.dot(point) + self.d
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub struct Frustum {
    left: Plane,
    right: Plane,
    top: Plane,
    bottom: Plane,
    near: Plane,
}

impl Frustum {
    pub fn from_matrix(matrix: Mat4) -> Self {
        let cols = matrix.to_cols_array_2d();

        let left = Plane::new(
            cols[0][3] + cols[0][0],
            cols[1][3] + cols[1][0],
            cols[2][3] + cols[2][0],
            cols[3][3] + cols[3][0],
        );

        let right = Plane::new(
            cols[0][3] - cols[0][0],
            cols[1][3] - cols[1][0],
            cols[2][3] - cols[2][0],
            cols[3][3] - cols[3][0],
        );

        let top = Plane::new(
            cols[0][3] - cols[0][1],
            cols[1][3] - cols[1][1],
            cols[2][3] - cols[2][1],
            cols[3][3] - cols[3][1],
        );

        let bottom = Plane::new(
            cols[0][3] + cols[0][1],
            cols[1][3] + cols[1][1],
            cols[2][3] + cols[2][1],
            cols[3][3] + cols[3][1],
        );

        let near = Plane::new(
            cols[0][3] - cols[0][2],
            cols[1][3] - cols[1][2],
            cols[2][3] - cols[2][2],
            cols[3][3] - cols[3][2],
        );

        Self {
            left: left.normalize(),
            right: right.normalize(),
            top: top.normalize(),
            bottom: bottom.normalize(),
            near: near.normalize(),
        }
    }

    pub fn contains_sphere(&self, sphere: Sphere) -> bool {
        let neg_radius = -sphere.radius;

        let array = [self.left, self.right, self.top, self.bottom, self.near];

        for plane in &array {
            let inside = plane.distance(sphere.center) >= neg_radius;
            if !inside {
                return false;
            }
        }

        true
    }
}

#[derive(QueryData)]
struct AabbDrawQuery {
    bb: &'static Aabb,
    transform: Option<&'static Transform>,
    filter: Option<&'static NodeFilter>,
    view_vis: Option<&'static ViewVisibility>,
}

#[derive(QueryFilter)]
struct AabbDrawFilter {
    _or: Or<(
        With<ShadowMapRenderer>,
        With<StaticModelSingle>,
        With<DynamicModelComponent>,
    )>,
}

// cohae: We keep these query types private to avoid cluttering the global namespace
#[allow(private_interfaces)]
pub fn draw_aabb_system(
    In(renderer): In<RendererShared>,
    q_aabb: Query<AabbDrawQuery, AabbDrawFilter>,
) {
    gpu_event!(renderer.gpu, "draw_aabb_system");

    for AabbDrawQueryItem {
        bb,
        transform,
        filter,
        view_vis,
    } in &q_aabb
    {
        let mut aabb_transform = Transform {
            translation: bb.center(),
            scale: bb.extents(),
            ..Default::default()
        }
        .local_to_world();
        if let Some(transform) = transform {
            aabb_transform = transform.local_to_world() * aabb_transform;
        }

        let mut color = filter.map(|f| f.color()).unwrap_or(Color::WHITE);
        if !view_vis.is_visible(0) {
            color = Color::from_rgba_premultiplied(color.r(), color.g(), color.b(), 0.2);
        }

        renderer
            .immediate
            .cube_extents(aabb_transform, color, false);

        let c = filter.map(|f| f.color()).unwrap_or(Color::WHITE);
        renderer.immediate.sphere(
            aabb_transform.to_scale_rotation_translation().2,
            bb.radius(),
            Color::from_rgba_premultiplied(c.r(), c.g(), c.b(), 0.1),
        )
    }
}
