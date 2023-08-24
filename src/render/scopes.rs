use glam::{Mat4, Vec4};

pub type Mat3x4 = [Vec4; 3];

// This scope uses official struct/field names from TFX intermediaries (scope_view)
#[repr(C)]
#[derive(Default)]
pub struct ScopeView {
    pub world_to_projective: Mat4,
    pub camera_to_world: Mat4,
    pub target_pixel_to_camera: Mat4,

    // pub target: Vec4,
    pub target_resolution: (f32, f32),
    pub inverse_target_resolution: (f32, f32),

    // pub view_miscellaneous: Vec4,
    pub maximum_depth_pre_projection: f32,
    pub view_is_first_person: f32,
    pub misc_unk2: f32,
    pub misc_unk3: f32,
}

// This scope uses official struct/field names from TFX intermediaries (scope_frame)
#[repr(C)]
#[derive(Default)]
pub struct ScopeFrame {
    // pub time: Vec4,               // c0
    pub game_time: f32,
    pub render_time: f32,
    pub delta_game_time: f32,
    pub exposure_time: f32,

    // pub exposure: Vec4,           // c1
    pub exposure_scale: f32,
    pub exposure_illum_relative_glow: f32,
    pub exposure_scale_for_shading: f32,
    pub exposure_illum_relative: f32,

    pub random_seed_scales: Vec4, // c2
    pub overrides: Vec4,          // c3
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ScopeInstances {
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
