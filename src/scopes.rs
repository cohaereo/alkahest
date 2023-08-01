use glam::{Mat4, Vec4};

pub type Mat3x4 = [Vec4; 3];

#[repr(C)]
#[derive(Default)]
// #[derive(Copy, Clone, Pod, Zeroable)]
pub struct ScopeView {
    pub world_to_projective: Mat4,
    pub camera_to_world: Mat4,

    // pub target_pixel_to_camera: Mat4
    pub _8: Vec4,
    pub _9: Vec4,
    pub _10: Vec4,
    pub _11: Vec4,
    // pub target: Vec4,
    pub _12: Vec4,
    // pub view_miscellaneous: Vec4,
    pub view_miscellaneous: Vec4,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
// #[derive(Copy, Clone, Pod, Zeroable)]
pub struct ScopeStaticInstance {
    pub mesh_to_world: Mat4,
    // /// Transform, Mat3x4?
    // pub _0: Mat3x4,
    // /// Texture coordinate transform (x yz) and flags (w)
    // pub _3: Vec4,
}
