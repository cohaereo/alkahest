use glam::{Mat4, Vec4};
use tiger_parse::tiger_type;
use tiger_pkg::TagHash;

#[tiger_type(id = 0x80807F3C, size = 0x1D8)]
#[derive(Debug, Clone)]
pub struct SCubemapComponent {
    pub unk0: u64,
    pub _pad8: u64,

    pub volume_extents: Vec4,
    pub volume_center: Vec4,

    pub unk30: f32,
    pub unk34: [u32; 3],
    pub unk40_fade1: Vec4,
    pub unk50_fade2: Vec4,

    pub unk60: Vec4,

    // pub unk70: f32,
    // pub unk74: f32,
    // pub unk78: u32,
    // pub unk7c: u32,

    // pub unk80: Vec4,
    // pub unk90: Vec4,
    // pub unka0: u32,
    // pub unka4: u32,
    // pub pada8: u64,
    #[tiger(offset = 0xc0)]
    pub unkc0: Mat4,
    pub unk100: Vec4,
    pub unk110: Vec4,
    pub unk120: Mat4,

    pub probes_resolution: [u32; 3],
    pub unk16c: f32,

    pub unk170: Vec4,
    pub unk180: Vec4,
    pub unk190: Vec4,
    pub unk1a0: [u64; 2],

    pub unk1b0: u32,
    pub unk1b4: TagHash,
    pub texture_cube_specular_ibl: TagHash,
    pub texture_cube_alpha: TagHash,
    pub texture_voxel_diffuse: TagHash,
    pub unk1c4_vb: TagHash,
    pub unk1c8: TagHash,
    pub unk1cc: [u32; 3],
}

#[derive(PartialEq)]
pub enum CubemapShape {
    Cube = 0,
    Sphere = 1,
    CubeSphere = 2,
}

impl SCubemapComponent {
    pub fn use_alpha(&self) -> bool {
        (self.unk180.z > 0.0) && self.unk180.w != 0.0
    }

    pub fn use_relighting(&self) -> bool {
        self.unk180.y > 0.0
    }

    pub fn use_probes(&self) -> bool {
        (self.probes_resolution[0] * self.probes_resolution[1] * self.probes_resolution[2]) > 1
    }

    // pub fn use_parallax(&self) -> bool {
    //     let v10 = self.unk90.abs();
    //     let mut v = Vec4::ZERO;
    //     v.x = (v10.x * 2.0) + self.unk80.x;
    //     v.y = (v10.y * 2.0) + self.unk80.y;
    //     v.z = (v10.z * 2.0) + self.unk80.z;
    //     v.w = self.unk80.w;

    //     v.x > 0.0 && v.y > 0.0 && v.z > 0.0
    // }

    pub fn shape(&self) -> CubemapShape {
        // // TODO(cohae): uses some more funky math to determine, but this is generally correct
        // if self.unk60.w > 0.0 {
        //     CubemapShape::CubeSphere
        // } else {
        //     CubemapShape::Cube
        // }
        CubemapShape::Cube
    }
}
