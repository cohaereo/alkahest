use binrw::binread;
use glam::{Quat, Vec4};

use super::{bvtree::BvTreeType, hkArrayIndex, hkPointerIndex};

#[binread]
#[derive(Debug)]
pub struct hkpStaticCompoundShape {
    pub unk0: [u32; 10],

    pub tree_type: BvTreeType,

    pub unk30: [u64; 2],

    pub instances: hkArrayIndex,

    pub unk48: [u64; 5],
    pub instance_extra_infos: hkArrayIndex,

    pub unk78: u64,

    #[br(map = Vec4::from_array)]
    pub tree_domain_min: Vec4,
    #[br(map = Vec4::from_array)]
    pub tree_domain_max: Vec4,
}

#[binread]
#[derive(Debug)]
pub struct hkpStaticCompoundShapeInstance {
    pub transform: hkQsTransform,
    pub shape: hkPointerIndex,

    pub unk28: [u32; 6],
}

#[binread]
#[derive(Debug)]
pub struct hkQsTransform {
    #[br(map = Vec4::from_array)]
    pub translation: Vec4,
    #[br(map = Vec4::from_array)]
    pub rotation: Vec4,
    #[br(map = Vec4::from_array)]
    pub scale: Vec4,
}

impl hkQsTransform {
    pub fn to_mat4(&self) -> glam::Mat4 {
        glam::Mat4::from_scale_rotation_translation(
            self.scale.truncate(),
            Quat::from_xyzw(
                self.rotation.x,
                self.rotation.y,
                self.rotation.z,
                self.rotation.w,
            ),
            self.translation.truncate(),
        )
    }
}
