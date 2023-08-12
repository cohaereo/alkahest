use glam::{Mat4, Vec4};

#[repr(C)]
#[derive(Default)]
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
    pub view_miscellaneous: Vec4,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ScopeStaticInstance {
    // TODO(cohae): Can we split this up into mesh_to_world, uv_transform and unk3c?
    pub mesh_to_world: Mat4,
}
