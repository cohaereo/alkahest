use std::io::Write;

use glam::{Mat4, Vec2, Vec3, Vec4};

pub type Mat3x4 = [Vec4; 3];

// This scope uses official struct/field names from TFX intermediaries (scope_view)
#[repr(C)]
pub struct ScopeView {
    pub world_to_projective: Mat4, // c0

    pub camera_to_world: Mat4, // c4
    // pub camera_right: Vec4,    // c4
    // pub camera_up: Vec4,       // c5
    // pub camera_backward: Vec4, // c6
    // pub camera_position: Vec4, // c7
    pub target_pixel_to_camera: Mat4, // c8

    // pub target: Vec4,
    pub target_resolution: (f32, f32), // c12
    pub inverse_target_resolution: (f32, f32),

    pub unk13: Vec4,
    pub view_miscellaneous: Vec4,
}

impl Default for ScopeView {
    fn default() -> Self {
        ScopeView {
            world_to_projective: Default::default(),
            camera_to_world: Default::default(),
            // camera_right: Default::default(),
            // camera_up: Default::default(),
            // camera_backward: Default::default(),
            // camera_position: Default::default(),
            target_pixel_to_camera: Default::default(),
            target_resolution: Default::default(),
            inverse_target_resolution: Default::default(),
            unk13: Vec4::new(0., 0., 2.622_604_4e-6, -1.0),
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

    // Light related
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
            exposure_time: 1. / 60.,
            exposure_scale: 1.0, // 0.5674781799316406,
            exposure_illum_relative_glow: 23.386_974,
            exposure_scale_for_shading: 0.567_478_2,
            exposure_illum_relative: 1.461_685_9,
            random_seed_scales: Vec4::new(102.850_5, 102.048_53, 943.289_06, 187.406_77),
            overrides: Vec4::new(0.5, 0.5, 0.0, 0.0),
            unk4: Vec4::new(1.0, 1.0, 0.0, 1.0),
            unk5: Vec4::new(0.0, f32::NAN, 512.0, 0.0),
            unk6: Vec4::new(0.0, 1.0, 0.966_787_6, 0.0),
            unk7: Vec4::new(0.0, 0.5, 180.0, 0.0),
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
    pub max_color_index: u32,

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
                f32::from_bits(self.max_color_index),
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
pub struct ScopeTransparent {
    pub depth_constants: Vec4,
    pub unk1: Vec4,
    pub unk2: Vec4,
    pub unk3: Vec4,
    pub unk4: Vec4,
    pub unk5: Vec4,
}

impl Default for ScopeTransparent {
    fn default() -> Self {
        ScopeTransparent {
            // transparent_scope_depth_constants: Vec4::new(0.0, 1.0, 1.0, 1.0),
            depth_constants: Vec4::new(1.785_714_3e-5, 50000.0, 0.0, 0.0),
            // depth_constants: Vec4::new(1.7857142665889114e-05, 6.666648864746094, 0.0, 0.0),
            unk1: Vec4::new(5.0, 2.718_281_7, 1.85, 100.0),
            unk2: Vec4::new(0.0, 0.0, -0.004_166_667_3, 0.020_833_336),
            unk3: Vec4::new(0.0, 0.0, -0.004_166_667_3, 0.020_833_336),
            unk4: Vec4::new(0.0, 0.0, -0.004_166_667_3, 0.020_833_336),
            unk5: Vec4::new(0.0, 0.0, 0.0, 0.0),
        }
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
            unk0: Vec4::W,
            unk1: Vec4::W,
            unk2: Vec4::W,
            unk3: Vec4::W,
            unk4: Vec4::W,
            unk5: Vec4::W,
            unk6: Vec4::W,
            unk7: Vec4::W,
            unk8: Vec4::W,
            unk9: Vec4::W,
            unk10: Vec4::W,
            unk11: Vec4::W,
            unk12: Vec4::W,
            unk13: Vec4::W,
            unk14: Vec4::W,
            unk15: Vec4::W,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ScopeTransparentAdvanced {
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

impl Default for ScopeTransparentAdvanced {
    fn default() -> Self {
        ScopeTransparentAdvanced {
            unk0: Vec4::new(
                0.000_984_931_4,
                0.001_983_686_8,
                0.000_778_356_7,
                0.001_558_671_2,
            ),
            unk1: Vec4::new(
                0.000_986_04,
                0.002_085_914,
                0.000_983_823_9,
                0.001_886_469_8,
            ),
            unk2: Vec4::new(
                0.001_186_082_4,
                0.002_434_628_8,
                0.000_946_840_8,
                0.001_850_187,
            ),
            unk3: Vec4::new(0.790_346_6, 0.731_906_4, 0.562_136_95, 0.0),
            unk4: Vec4::new(0.0, 1.0, 0.109375, 0.046875),
            unk5: Vec4::new(0.0, 0.0, 0.0, 0.000_869_452_95),
            unk6: Vec4::new(0.55, 0.410_910_52, 0.226_709_46, 0.503_812_73),
            unk7: Vec4::new(1.0, 1.0, 1.0, 0.999_777_8),
            unk8: Vec4::new(132.928_85, 66.404_44, 56.853_416, 0.0),
            unk9: Vec4::new(132.928_85, 66.404_44, 1000.0, 1e-4),
            unk10: Vec4::new(131.928_85, 65.404_44, 55.853_416, 0.678_431_4),
            unk11: Vec4::new(131.928_85, 65.404_44, 999.0, 5.5),
            unk12: Vec4::new(0.0, 0.5, 25.575_994, 0.0),
            unk13: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk14: Vec4::new(0.025, 10000.0, -9999.0, 1.0),
            unk15: Vec4::new(1.0, 1.0, 1.0, 0.0),
            unk16: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk17: Vec4::new(10.979_255, 7.148_235_3, 6.303_493_5, 0.0),
            unk18: Vec4::new(0.003_761_407_2, 0.0, 0.0, 0.0),
            unk19: Vec4::new(0.0, 0.007_529_612_6, 0.0, 0.0),
            unk20: Vec4::new(0.0, 0.0, 0.017_589_089, 0.0),
            unk21: Vec4::new(0.272_664_84, -0.314_738_18, -0.156_036_81, 1.0),
            unk22: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk23: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk24: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk25: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk26: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk27: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk28: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk29: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk30: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk31: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk32: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk33: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk34: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk35: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk36: Vec4::new(1.0, 0.0, 0.0, 0.0),
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
