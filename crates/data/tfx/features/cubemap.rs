use glam::{Mat4, Vec4};
use tiger_parse::tiger_type;
use tiger_pkg::TagHash;

#[tiger_type(id = 0x80806695, size = 0x1B8)]
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
    pub unk70: f32,

    // pub unk74: f32,
    // pub unk78: u32,
    // pub unk7c: u32,

    // pub unk80: Vec4,
    // pub unk90: Vec4,
    // pub unka0: u32,
    // pub unka4: u32,
    // pub pada8: u64,
    #[tiger(offset = 0xb0)]
    pub unkb0: Mat4,
    pub unkf0: Vec4,
    pub unk100: Vec4,

    #[tiger(offset = 0x110)]
    pub unk110: Mat4,

    pub probes_resolution: [u32; 3],
    pub unk15c: f32,

    pub unk160: Vec4,
    pub unk170: Vec4,
    pub unk180: Vec4,

    pub unk190: u64,
    pub unk198: u16,
    pub unk19a: u16,
    pub texture_cube_specular_ibl: TagHash,
    pub texture_cube_alpha: TagHash,
    pub texture_voxel_diffuse: TagHash,
    pub unk1a8: TagHash,
    pub unk1ac: [u32; 3],
}

#[derive(PartialEq)]
pub enum CubemapShape {
    Cube = 0,
    Sphere = 1,
    CubeSphere = 2,
}

impl SCubemapComponent {
    pub fn use_alpha(&self) -> bool {
        (self.unk170.z > 0.0) && self.unk170.w != 0.0
    }

    pub fn use_relighting(&self) -> bool {
        self.unk170.y > 0.0
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
