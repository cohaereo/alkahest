use glam::{Mat4, Vec4};

pub type Mat3x4 = [Vec4; 3];

// This scope uses official struct/field names from TFX intermediaries (scope_view)
#[repr(C)]
pub struct ScopeView {
    pub world_to_projective: Mat4,

    // pub camera_to_world: mat4
    pub camera_right: Vec4,
    pub camera_up: Vec4,
    pub camera_backward: Vec4,
    pub camera_position: Vec4,

    pub target_pixel_to_camera: Mat4,

    // pub target: Vec4,
    pub target_resolution: (f32, f32),
    pub inverse_target_resolution: (f32, f32),

    // pub view_miscellaneous: Vec4,
    pub misc_unk0: f32,
    pub misc_unk1: f32,
    pub misc_unk2: f32,
    pub misc_unk3: f32,
}

impl Default for ScopeView {
    fn default() -> Self {
        ScopeView {
            world_to_projective: Default::default(),
            camera_right: Default::default(),
            camera_up: Default::default(),
            camera_backward: Default::default(),
            camera_position: Default::default(),
            target_pixel_to_camera: Default::default(),
            target_resolution: Default::default(),
            inverse_target_resolution: Default::default(),
            misc_unk0: Default::default(),
            misc_unk1: Default::default(),
            misc_unk2: 0.0001,
            misc_unk3: Default::default(),
        }
    }
}

// This scope uses official struct/field names from TFX intermediaries (scope_frame)
#[repr(C)]
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

    pub unk4: Vec4, // c4
    pub unk5: Vec4, // c5
    pub unk6: Vec4, // c6
    pub unk7: Vec4, // c7
}

impl Default for ScopeFrame {
    fn default() -> Self {
        ScopeFrame {
            game_time: Default::default(),
            render_time: Default::default(),
            delta_game_time: Default::default(),
            exposure_time: Default::default(),
            exposure_scale: 1.0,
            exposure_illum_relative_glow: 1.0,
            exposure_scale_for_shading: 0.0,
            exposure_illum_relative: 0.0,
            random_seed_scales: Default::default(),
            overrides: Default::default(),
            unk4: Default::default(),
            unk5: Default::default(),
            unk6: Default::default(),
            unk7: Default::default(),
        }
    }
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

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ScopeUnk2 {
    pub unk0: Vec4,
    pub unk1: Vec4,
    pub unk2: Vec4,
    pub unk3: Vec4,
    pub unk4: Vec4,
    pub unk5: Vec4,
}

impl Default for ScopeUnk2 {
    fn default() -> Self {
        ScopeUnk2 {
            unk0: Vec4::ONE,
            unk1: Vec4::ONE,
            unk2: Vec4::ONE,
            unk3: Vec4::ONE,
            unk4: Vec4::ONE,
            unk5: Vec4::ONE,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ScopeUnk8 {
    pub unk0: Vec4,
    pub unk1: Vec4,
    pub unk2: Vec4,
    pub unk3: Vec4,
    pub unk4: Vec4,
    pub unk5: Vec4,
    pub unk6: Vec4,
    pub unk7: Vec4,
}

impl Default for ScopeUnk8 {
    fn default() -> Self {
        ScopeUnk8 {
            unk0: Vec4::ONE,
            unk1: Vec4::ONE,
            unk2: Vec4::ONE,
            unk3: Vec4::ONE,
            unk4: Vec4::ONE,
            unk5: Vec4::ONE,
            unk6: Vec4::ONE,
            unk7: Vec4::ONE,
        }
    }
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
