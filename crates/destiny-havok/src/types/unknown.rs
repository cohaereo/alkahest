// Unknown struct purgatory

use binrw::binread;
use glam::Vec4;

use super::{hkArrayIndex, hkPointerIndex};

#[binread]
#[derive(Debug)]
pub struct Unk81 {
    pub unk0: [u64; 7],
    pub unk38: hkArrayIndex,
    pub unk40: [u64; 2],

    #[br(map = Vec4::from_array)]
    pub unk50: Vec4,
    #[br(map = Vec4::from_array)]
    pub unk60: Vec4,

    pub unk70: [u64; 4],
}

#[binread]
#[derive(Debug)]
pub struct Unk84 {
    pub shape: hkPointerIndex,
    pub unk8: [u64; 3],
}
