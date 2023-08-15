use glam::{Mat4, Vec4};

pub type Mat3x4 = [Vec4; 3];

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
    pub mesh_to_world: Mat3x4,
    pub texcoord_transform: Vec4,
}

// This scope uses official struct/field names from TFX intermediaries (scope_rigid_model)
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ScopeRigidModel {
    pub mesh_to_world: Mat4,          // c0
    pub position_scale: Vec4,         // c4
    pub position_offset: Vec4,        // c5
    pub texcoord0_scale_offset: Vec4, // c6
    pub dynamic_sh_ao_values: Vec4,   // c7
}

pub trait MatrixConversion {
    /// Truncates/extends the given matrix to 3 rows, 4 columns
    fn to_3x4(&self) -> Mat3x4;
}

impl MatrixConversion for Mat4 {
    fn to_3x4(&self) -> Mat3x4 {
        [self.x_axis, self.y_axis, self.z_axis]
    }
}
