use std::io::Cursor;

use binrw::{binread, BinReaderExt, Endian};

use super::externs::TfxExtern;

#[rustfmt::skip]
#[binread]
#[derive(Debug)]
pub enum TfxBytecodeOp {
    #[br(magic = 0x01_u8)] Unk01,
    #[br(magic = 0x02_u8)] Unk02,
    #[br(magic = 0x03_u8)] Unk03,
    #[br(magic = 0x04_u8)] Unk04,
    #[br(magic = 0x09_u8)] Unk09 { unk1: u8 },
    #[br(magic = 0x0b_u8)] Unk0b { unk1: u8, unk2: u8 },
    #[br(magic = 0x0c_u8)] Unk0c,
    #[br(magic = 0x0d_u8)] Unk0d,
    #[br(magic = 0x0e_u8)] Unk0e,
    #[br(magic = 0x0f_u8)] Unk0f,
    #[br(magic = 0x10_u8)] Unk10,
    #[br(magic = 0x12_u8)] Unk12,
    #[br(magic = 0x17_u8)] Unk17,
    #[br(magic = 0x1a_u8)] Unk1a,
    #[br(magic = 0x1c_u8)] Unk1c,
    #[br(magic = 0x1d_u8)] Unk1d,
    #[br(magic = 0x1f_u8)] Unk1f,
    #[br(magic = 0x20_u8)] Unk20,
    #[br(magic = 0x21_u8)] Unk21,
    #[br(magic = 0x22_u8)] Unk22 { unk1: u8 },
    #[br(magic = 0x23_u8)] Unk23,
    #[br(magic = 0x26_u8)] Unk26,
    #[br(magic = 0x27_u8)] Unk27,
    #[br(magic = 0x28_u8)] Unk28,
    #[br(magic = 0x29_u8)] Unk29,
    #[br(magic = 0x2a_u8)] Unk2a,
    #[br(magic = 0x2e_u8)] Unk2e,
    #[br(magic = 0x34_u8)] UnkLoadConstant2 { constant_index: u8 },
    #[br(magic = 0x35_u8)] Unk35 { unk1: u8 },
    #[br(magic = 0x37_u8)] Unk37 { unk1: u8 },
    #[br(magic = 0x38_u8)] Unk38 { unk1: u8 },
    #[br(magic = 0x39_u8)] Unk39 { unk1: u8 },
    #[br(magic = 0x3a_u8)] Unk3a { unk1: u8 },
    #[br(magic = 0x3b_u8)] UnkLoadConstant { constant_index: u8 },
    #[br(magic = 0x3c_u8)] LoadExtern { extern_: TfxExtern, element: u8 },
    #[br(magic = 0x3d_u8)] Unk3d { unk1: u8, unk2: u8 },
    #[br(magic = 0x3e_u8)] Unk3e { unk1: u8 },
    #[br(magic = 0x3f_u8)] Unk3f { unk1: u8, unk2: u8 },
    #[br(magic = 0x42_u8)] Unk42 { unk1: u8, unk2: u8 },
    #[br(magic = 0x43_u8)] StoreToBuffer { element: u8 },
    #[br(magic = 0x45_u8)] Unk45 { unk1: u8 },
    #[br(magic = 0x46_u8)] Unk46 { unk1: u8 },
    #[br(magic = 0x47_u8)] Unk47 { unk1: u8 },
    #[br(magic = 0x49_u8)] Unk49 { unk1: u8 },
    #[br(magic = 0x4c_u8)] Unk4c { unk1: u8 },
    #[br(magic = 0x4d_u8)] Unk4d { unk1: u8 },
    #[br(magic = 0x4e_u8)] Unk4e { unk1: u8 },
}

impl TfxBytecodeOp {
    pub fn parse_all(data: &[u8], endian: Endian) -> binrw::BinResult<Vec<TfxBytecodeOp>> {
        let mut cur = Cursor::new(data);
        let mut opcodes = vec![];

        while (cur.position() as usize) < data.len() {
            let op = cur.read_type::<TfxBytecodeOp>(endian)?;
            opcodes.push(op);
        }

        Ok(opcodes)
    }
}
