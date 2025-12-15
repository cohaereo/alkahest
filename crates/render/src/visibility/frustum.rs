use std::ops::Deref;

use alkahest_data::tfx::common::AxisAlignedBBox;
use glam::{vec4, Vec3A, Vec4, Vec4Swizzles};

use crate::camera::Camera;

#[derive(Default)]
pub struct Plane(pub Vec4);

impl Plane {
    pub fn normal(&self) -> Vec3A {
        self.xyz().into()
    }

    pub fn intersect(p0: &Self, p1: &Self, p2: &Self) -> Vec3A {
        let bxc = p1.normal().cross(p2.normal());
        let cxa = p2.normal().cross(p0.normal());
        let axb = p0.normal().cross(p1.normal());
        let r = -bxc * p0.w - cxa * p1.w - axb * p2.w;
        r * (1.0 / bxc.dot(p0.normal()))
    }

    pub fn normalized(&self) -> Self {
        Plane(self.0 / self.xyz().length())
    }

    pub fn get_signed_distance(&self, point: Vec3A) -> f32 {
        self.normal().dot(point) + self.w
    }
}

impl Deref for Plane {
    type Target = Vec4;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Default)]
pub struct Frustum {
    pub planes: [Plane; 6],
    pub points: [Vec3A; 8],
}

impl Frustum {
    const LEFT: usize = 0;
    const RIGHT: usize = 1;
    const BOTTOM: usize = 2;
    const TOP: usize = 3;
    const NEAR: usize = 4;
    const FAR: usize = 5;

    #[rustfmt::skip]
    pub fn new(planes: [Plane; 6]) -> Self {
        let points = [
            Plane::intersect(&planes[Self::NEAR], &planes[Self::LEFT], &planes[Self::BOTTOM]),
            Plane::intersect(&planes[Self::NEAR], &planes[Self::LEFT], &planes[Self::TOP]),
            Plane::intersect(&planes[Self::NEAR], &planes[Self::RIGHT], &planes[Self::TOP]),
            Plane::intersect(&planes[Self::NEAR], &planes[Self::RIGHT], &planes[Self::BOTTOM]),
            Plane::intersect(&planes[Self::FAR], &planes[Self::LEFT], &planes[Self::BOTTOM]),
            Plane::intersect(&planes[Self::FAR], &planes[Self::LEFT], &planes[Self::TOP]),
            Plane::intersect(&planes[Self::FAR], &planes[Self::RIGHT], &planes[Self::TOP]),
            Plane::intersect(&planes[Self::FAR], &planes[Self::RIGHT], &planes[Self::BOTTOM]),
        ];

        Self { planes, points }
    }

    pub fn from_camera(cam: &Camera) -> Self {
        let cols = (cam.projection_matrix_standard() * cam.view_matrix()).to_cols_array_2d();

        let left = Plane(vec4(
            cols[0][3] + cols[0][0],
            cols[1][3] + cols[1][0],
            cols[2][3] + cols[2][0],
            cols[3][3] + cols[3][0],
        ));

        let right = Plane(vec4(
            cols[0][3] - cols[0][0],
            cols[1][3] - cols[1][0],
            cols[2][3] - cols[2][0],
            cols[3][3] - cols[3][0],
        ));

        let top = Plane(vec4(
            cols[0][3] - cols[0][1],
            cols[1][3] - cols[1][1],
            cols[2][3] - cols[2][1],
            cols[3][3] - cols[3][1],
        ));

        let bottom = Plane(vec4(
            cols[0][3] + cols[0][1],
            cols[1][3] + cols[1][1],
            cols[2][3] + cols[2][1],
            cols[3][3] + cols[3][1],
        ));

        let near = Plane(vec4(cols[0][2], cols[1][2], cols[2][2], cols[3][2]));

        let far = Plane(vec4(
            cols[0][3] - cols[0][2],
            cols[1][3] - cols[1][2],
            cols[2][3] - cols[2][2],
            cols[3][3] - cols[3][2],
        ));

        Self::new([left, right, bottom, top, near, far])
    }

    pub fn aabb_intersecting(&self, bb: &AxisAlignedBBox) -> bool {
        let extents: Vec3A = bb.extents().into();
        let center: Vec3A = bb.center().into();
        let is_on_or_forward_plane = |plane: &Plane| {
            let r = extents.dot(Vec3A::from(plane.xyz()).abs());
            // let r =
            //     extents.x * plane.x.abs() + extents.y * plane.y.abs() + extents.z * plane.z.abs();
            -r <= plane.get_signed_distance(center)
        };

        self.planes.iter().all(is_on_or_forward_plane)
    }

    // pub fn sphere_intersecting(&self, center: impl Into<Vec3A>, radius: f32) -> bool {
    //     let center = center.into();
    //     for plane in &self.planes {
    //         if plane.get_signed_distance(center) < -radius {
    //             return false;
    //         }
    //     }
    //     true
    // }
}
