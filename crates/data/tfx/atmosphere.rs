use tiger_parse::tiger_type;
use tiger_pkg::TagHash;

use crate::tag::WideHash;

#[tiger_type(id = 0x80806BC1)]
pub struct SAtmosphereDataComponent {
    pub unk0: [u32; 4 * 8],
    pub unk80_tex: WideHash,
    pub unk90_tex: WideHash,
    pub unka0_tex: WideHash,
    pub unkb0_tex: WideHash,
    pub unkc0_tex: TagHash,
    pub unkc4: TagHash,

    pub unkc8: [f32; 4 * 4],
}
