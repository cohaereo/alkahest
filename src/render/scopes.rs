use std::io::Write;

use glam::{Mat4, Vec2, Vec3, Vec4};

pub type Mat3x4 = [Vec4; 3];

// This scope uses official struct/field names from TFX intermediaries (scope_view)
#[repr(C)]
pub struct ScopeView {
    pub world_to_projective: Mat4, // c0

    // pub camera_to_world: mat4
    pub camera_right: Vec4,    // c4
    pub camera_up: Vec4,       // c5
    pub camera_backward: Vec4, // c6
    pub camera_position: Vec4, // c7

    pub target_pixel_to_camera: Mat4, // c8

    // pub target: Vec4,
    pub target_resolution: (f32, f32), // c12
    pub inverse_target_resolution: (f32, f32),

    pub unk13: f32,
    pub view_miscellaneous: Vec4,
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
            unk13: 1.0,
            view_miscellaneous: Vec4::new(0.0, 0.0, 0.0001, 0.0),
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
#[derive(Clone, Default)]
pub struct ScopeInstances {
    pub mesh_offset: Vec3,
    pub mesh_scale: f32,
    pub uv_scale: f32,
    pub uv_offset: Vec2,
    pub unk1_w: u32,

    pub transforms: Vec<Mat4>,
}

impl ScopeInstances {
    pub fn write(&self) -> Vec<u8> {
        let mut buffer = vec![];

        buffer
            .write_all(bytemuck::cast_slice(&[
                self.mesh_offset.x,
                self.mesh_offset.y,
                self.mesh_offset.z,
                self.mesh_scale,
                self.uv_scale,
                self.uv_offset.x,
                self.uv_offset.y,
                f32::from_bits(self.unk1_w),
            ]))
            .unwrap();

        buffer
            .write_all(bytemuck::cast_slice(&self.transforms))
            .unwrap();

        buffer
    }
}

// This scope uses official struct/field names from TFX intermediaries (scope_rigid_model)
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ScopeRigidModel {
    pub mesh_to_world: Mat4,          // c0
    pub position_scale: Vec4,         // c4
    pub position_offset: Vec4,        // c5
    pub texcoord0_scale_offset: Vec4, // c6
    pub dynamic_sh_ao_values: Vec4,   // c7
    pub unk8: [Mat4; 8],              // c8
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ScopeUnk2 {
    pub unk0: Vec4,
}

impl Default for ScopeUnk2 {
    fn default() -> Self {
        ScopeUnk2 { unk0: Vec4::ONE }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ScopeUnk3 {
    pub unk0: Vec4,
    pub unk1: Vec4,
    pub unk2: Vec4,
    pub unk3: Vec4,
    pub unk4: Vec4,
    pub unk5: Vec4,
    pub unk6: Vec4,
    pub unk7: Vec4,
    pub unk8: Vec4,
    pub unk9: Vec4,
    pub unk10: Vec4,
    pub unk11: Vec4,
    pub unk12: Vec4,
    pub unk13: Vec4,
    pub unk14: Vec4,
    pub unk15: Vec4,
}

impl Default for ScopeUnk3 {
    fn default() -> Self {
        ScopeUnk3 {
            unk0: Vec4::ONE,
            unk1: Vec4::ONE,
            unk2: Vec4::ONE,
            unk3: Vec4::ONE,
            unk4: Vec4::ONE,
            unk5: Vec4::ONE,
            unk6: Vec4::ONE,
            unk7: Vec4::ONE,
            unk8: Vec4::ONE,
            unk9: Vec4::ONE,
            unk10: Vec4::ONE,
            unk11: Vec4::ONE,
            unk12: Vec4::ONE,
            unk13: Vec4::ONE,
            unk14: Vec4::ONE,
            unk15: Vec4::ONE,
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
    pub unk8: Vec4,
    pub unk9: Vec4,
    pub unk10: Vec4,
    pub unk11: Vec4,
    pub unk12: Vec4,
    pub unk13: Vec4,
    pub unk14: Vec4,
    pub unk15: Vec4,
    pub unk16: Vec4,
    pub unk17: Vec4,
    pub unk18: Vec4,
    pub unk19: Vec4,
    pub unk20: Vec4,
    pub unk21: Vec4,
    pub unk22: Vec4,
    pub unk23: Vec4,
    pub unk24: Vec4,
    pub unk25: Vec4,
    pub unk26: Vec4,
    pub unk27: Vec4,
    pub unk28: Vec4,
    pub unk29: Vec4,
    pub unk30: Vec4,
    pub unk31: Vec4,
    pub unk32: Vec4,
    pub unk33: Vec4,
    pub unk34: Vec4,
    pub unk35: Vec4,
    pub unk36: Vec4,
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
            unk8: Vec4::ONE,
            unk9: Vec4::ONE,
            unk10: Vec4::ONE,
            unk11: Vec4::ONE,
            unk12: Vec4::ONE,
            unk13: Vec4::ONE,
            unk14: Vec4::ONE,
            unk15: Vec4::ONE,
            unk16: Vec4::ONE,
            unk17: Vec4::ONE,
            unk18: Vec4::ONE,
            unk19: Vec4::ONE,
            unk20: Vec4::ONE,
            unk21: Vec4::ONE,
            unk22: Vec4::ONE,
            unk23: Vec4::ONE,
            unk24: Vec4::ONE,
            unk25: Vec4::ONE,
            unk26: Vec4::ONE,
            unk27: Vec4::ONE,
            unk28: Vec4::ONE,
            unk29: Vec4::ONE,
            unk30: Vec4::ONE,
            unk31: Vec4::ONE,
            unk32: Vec4::ONE,
            unk33: Vec4::ONE,
            unk34: Vec4::ONE,
            unk35: Vec4::ONE,
            unk36: Vec4::ONE,
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
