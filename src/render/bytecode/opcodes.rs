use std::io::Cursor;

use binrw::{binread, BinReaderExt, Endian};

use super::externs::TfxExtern;

#[rustfmt::skip]
#[binread]
#[derive(Debug)]
pub enum TfxBytecodeOp {
    #[br(magic = 0x01_u8)] Add,
    #[br(magic = 0x02_u8)] Subtract,
    #[br(magic = 0x03_u8)] Multiply,
    #[br(magic = 0x04_u8)] IsZero,
    #[br(magic = 0x05_u8)] Multiply2, // TODO(cohae): Same as multiply? Might just be an alias used for division
    #[br(magic = 0x06_u8)] Add2,
    #[br(magic = 0x07_u8)] Unk07,
    #[br(magic = 0x08_u8)] Min,
    #[br(magic = 0x09_u8)] Max,
    #[br(magic = 0x0a_u8)] Unk0a,
    #[br(magic = 0x0b_u8)] Unk0b,
    #[br(magic = 0x0c_u8)] Merge1_3, // merge_1_3?
    #[br(magic = 0x0d_u8)] Unk0d,
    #[br(magic = 0x0e_u8)] Unk0e,
    #[br(magic = 0x0f_u8)] Unk0f,
    #[br(magic = 0x10_u8)] Unk10,
    #[br(magic = 0x11_u8)] Unk11,
    #[br(magic = 0x12_u8)] MultiplyAdd,
    #[br(magic = 0x13_u8)] Clamp,
    #[br(magic = 0x14_u8)] Unk14,
    #[br(magic = 0x15_u8)] Unk15,
    #[br(magic = 0x16_u8)] Unk16,
    #[br(magic = 0x17_u8)] Unk17,
    #[br(magic = 0x18_u8)] Unk18,
    #[br(magic = 0x19_u8)] Unk19,
    #[br(magic = 0x1a_u8)] Unk1a,
    #[br(magic = 0x1b_u8)] Unk1b,
    #[br(magic = 0x1c_u8)] Unk1c,
    #[br(magic = 0x1d_u8)] Negate,
    #[br(magic = 0x1f_u8)] Cosine, // cosine?
    #[br(magic = 0x20_u8)] Unk20,
    #[br(magic = 0x21_u8)] PermuteAllX, // Alias for permute(.xxxx) (TODO: confirm)
    #[br(magic = 0x22_u8)] Permute { fields: u8 }, // permute (TODO: how does the value work? we only see 0 = .xxxx in talk bytecode)
    #[br(magic = 0x23_u8)] Saturate, // saturate?
    #[br(magic = 0x24_u8)] Unk24,
    #[br(magic = 0x25_u8)] Unk25,
    #[br(magic = 0x26_u8)] Unk26,
    #[br(magic = 0x27_u8)] Unk27, // triangle???
    #[br(magic = 0x28_u8)] Unk28, // jitter?
    #[br(magic = 0x29_u8)] Unk29, // wander?
    #[br(magic = 0x2a_u8)] Unk2a, // rand?
    #[br(magic = 0x2b_u8)] Unk2b, // rand_smooth?
    #[br(magic = 0x2c_u8)] Unk2c,
    #[br(magic = 0x2d_u8)] Unk2d,
    #[br(magic = 0x2e_u8)] Unk2e,
    #[br(magic = 0x34_u8)] PushConstVec4 { constant_index: u8 }, // push_const_vec4?
    #[br(magic = 0x35_u8)] Unk35 { unk1: u8 },
    #[br(magic = 0x37_u8)] Unk37 { unk1: u8 }, // spline4_const?
    #[br(magic = 0x38_u8)] Unk38 { unk1: u8 },
    #[br(magic = 0x39_u8)] Unk39 { unk1: u8 },
    #[br(magic = 0x3a_u8)] Unk3a { unk1: u8 },
    #[br(magic = 0x3b_u8)] UnkLoadConstant { constant_index: u8 },
    /// Pushes an extern float to the stack, extended to all 4 elements (value.xxxx)
    /// Offset is in single floats (4 bytes)
    #[br(magic = 0x3c_u8)] PushExternInputFloat { extern_: TfxExtern, offset: u8 },
    /// Pushes an extern vec4 to the stack
    /// Offset is in vec4s (16 bytes)
    #[br(magic = 0x3d_u8)] Unk3d { extern_: TfxExtern, offset: u8 },
    // TODO(cohae): from first glance this looks like a weird copy of 0x3d
    #[br(magic = 0x3e_u8)] Unk3e { extern_: TfxExtern, offset: u8 },
    /// Pushes an extern vec2 to the stack
    /// Offset is in vec2s (8 bytes)
    #[br(magic = 0x3f_u8)] Unk3f { extern_: TfxExtern, offset: u8 },
    #[br(magic = 0x40_u8)] Unk40 { extern_: TfxExtern, offset: u8 },
    // TODO(cohae): Carbon copy of 0x3f???
    #[br(magic = 0x41_u8)] Unk41 { extern_: TfxExtern, offset: u8 },
    // TODO(cohae): Loads a value from the interpreter state + 0x44a0
    #[br(magic = 0x42_u8)] Unk42,
    #[br(magic = 0x43_u8)] Unk43 { unk1: u8 },
    #[br(magic = 0x44_u8)] PopOutput { element: u8 }, // pop_output? 0x43 in bytecode from talk
    #[br(magic = 0x45_u8)] Unk45 { slot: u8 },
    #[br(magic = 0x46_u8)] PushTemp { slot: u8 },
    #[br(magic = 0x47_u8)] PopTemp { slot: u8 },
    #[br(magic = 0x48_u8)] Unk48 { unk1: u8 },
    #[br(magic = 0x49_u8)] Unk49 { unk1: u8 },
    #[br(magic = 0x4a_u8)] Unk4a { unk1: u8 }, // Has conditional execution (unk1=1/2/3/4/5/6)
    #[br(magic = 0x4b_u8)] Unk4b { unk1: u8 },
    #[br(magic = 0x4c_u8)] Unk4c { unk1: u8 },
    #[br(magic = 0x4d_u8)] Unk4d { unk1: u8 }, // push_object_channel_vector? (push_object_channel_*??)
    #[br(magic = 0x4e_u8)] Unk4e { unk1: u8, unk2: u8, unk3: u8, unk4: u8 },
    #[br(magic = 0x4f_u8)] Unk4f { unk1: u8 },
    #[br(magic = 0x50_u8)] Unk50 { unk1: u8 },
    #[br(magic = 0x51_u8)] Unk51,
    #[br(magic = 0x52_u8)] Unk52 { unk1: u8, unk2: u8 },
    #[br(magic = 0x53_u8)] Unk53 { unk1: u8, unk2: u8 },
    #[br(magic = 0x54_u8)] Unk54 { unk1: u8, unk2: u8 },
    #[br(magic = 0x55_u8)] Unk55,
    #[br(magic = 0x56_u8)] Unk56,
    #[br(magic = 0x57_u8)] Unk57,
    #[br(magic = 0x58_u8)] Unk58,
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

    /// Formats the opcode to assembly-like output
    pub fn disassemble(&self) -> String {
        match self {
            TfxBytecodeOp::Add => "add".to_string(),
            TfxBytecodeOp::Subtract => "subtract".to_string(),
            TfxBytecodeOp::Multiply => "multiply".to_string(),
            TfxBytecodeOp::IsZero => "is_zero".to_string(),
            TfxBytecodeOp::Multiply2 => "multiply2".to_string(),
            TfxBytecodeOp::Add2 => "add2".to_string(),
            TfxBytecodeOp::Unk07 => "unk07".to_string(),
            TfxBytecodeOp::Min => "min".to_string(),
            TfxBytecodeOp::Max => "max".to_string(),
            TfxBytecodeOp::Unk0a => "unk0a".to_string(),
            TfxBytecodeOp::Unk0b => "unk0b".to_string(),
            TfxBytecodeOp::Merge1_3 => "merge_1_3".to_string(),
            TfxBytecodeOp::Unk0d => "unk0d".to_string(),
            TfxBytecodeOp::Unk0e => "unk0e".to_string(),
            TfxBytecodeOp::Unk0f => "unk0f".to_string(),
            TfxBytecodeOp::Unk10 => "unk10".to_string(),
            TfxBytecodeOp::Unk11 => "unk11".to_string(),
            TfxBytecodeOp::MultiplyAdd => "multiply_add".to_string(),
            TfxBytecodeOp::Clamp => "clamp".to_string(),
            TfxBytecodeOp::Unk14 => "unk14".to_string(),
            TfxBytecodeOp::Unk15 => "unk15".to_string(),
            TfxBytecodeOp::Unk16 => "unk16".to_string(),
            TfxBytecodeOp::Unk17 => "unk17".to_string(),
            TfxBytecodeOp::Unk18 => "unk18".to_string(),
            TfxBytecodeOp::Unk19 => "unk19".to_string(),
            TfxBytecodeOp::Unk1a => "unk1a".to_string(),
            TfxBytecodeOp::Unk1b => "unk1b".to_string(),
            TfxBytecodeOp::Unk1c => "unk1c".to_string(),
            TfxBytecodeOp::Negate => "negate".to_string(),
            TfxBytecodeOp::Cosine => "cosine".to_string(),
            TfxBytecodeOp::Unk20 => "unk20".to_string(),
            TfxBytecodeOp::PermuteAllX => "permute(.xxxx) (permute_all_x)".to_string(),
            TfxBytecodeOp::Permute { fields } => {
                format!("permute({})", decode_permute_param(*fields))
            }
            TfxBytecodeOp::Saturate => "saturate".to_string(),
            TfxBytecodeOp::Unk24 => "unk24".to_string(),
            TfxBytecodeOp::Unk25 => "unk25".to_string(),
            TfxBytecodeOp::Unk26 => "unk26".to_string(),
            TfxBytecodeOp::Unk27 => "unk27".to_string(),
            TfxBytecodeOp::Unk28 => "unk28".to_string(),
            TfxBytecodeOp::Unk29 => "unk29".to_string(),
            TfxBytecodeOp::Unk2a => "unk2a".to_string(),
            TfxBytecodeOp::Unk2b => "unk2b".to_string(),
            TfxBytecodeOp::Unk2c => "unk2c".to_string(),
            TfxBytecodeOp::Unk2d => "unk2d".to_string(),
            TfxBytecodeOp::Unk2e => "unk2e".to_string(),
            TfxBytecodeOp::PushConstVec4 { constant_index } => {
                format!("push_const_vec4({constant_index})")
            }
            TfxBytecodeOp::Unk35 { unk1 } => {
                format!("unk35 unk1={unk1}")
            }
            TfxBytecodeOp::Unk37 { unk1 } => {
                format!("unk37 unk1={unk1}")
            }
            TfxBytecodeOp::Unk38 { unk1 } => {
                format!("unk38 unk1={unk1}")
            }
            TfxBytecodeOp::Unk39 { unk1 } => {
                format!("unk39 unk1={unk1}")
            }
            TfxBytecodeOp::Unk3a { unk1 } => {
                format!("unk3a unk1={unk1}")
            }
            TfxBytecodeOp::UnkLoadConstant { constant_index } => {
                format!("unk_load_constant constants[{constant_index}]")
            }
            TfxBytecodeOp::PushExternInputFloat { extern_, offset } => {
                format!("push_extern_input_float ({extern_:?}+{offset})")
            }
            TfxBytecodeOp::Unk3d { extern_, offset } => {
                format!("unk3d extern={extern_:?} offset={offset}")
            }
            TfxBytecodeOp::Unk3e { extern_, offset } => {
                format!("unk3e extern={extern_:?} offset={offset}")
            }
            TfxBytecodeOp::Unk3f { extern_, offset } => {
                format!("unk3f extern={extern_:?} offset={offset}")
            }
            TfxBytecodeOp::Unk40 { extern_, offset } => {
                format!("unk40 extern={extern_:?} offset={offset}")
            }
            TfxBytecodeOp::Unk41 { extern_, offset } => {
                format!("unk41 extern={extern_:?} offset={offset}")
            }
            TfxBytecodeOp::Unk42 => "unk42".to_string(),
            TfxBytecodeOp::Unk43 { unk1 } => {
                format!("unk43 unk1={unk1}")
            }
            TfxBytecodeOp::PopOutput { element } => {
                format!("pop_output({element})")
            }
            TfxBytecodeOp::Unk45 { slot } => {
                format!("unk45 unk1={slot})")
            }
            TfxBytecodeOp::PushTemp { slot } => {
                format!("push_temp({slot})")
            }
            TfxBytecodeOp::PopTemp { slot } => {
                format!("pop_temp({slot})")
            }
            TfxBytecodeOp::Unk48 { unk1 } => {
                format!("unk48 unk1={unk1}")
            }
            TfxBytecodeOp::Unk49 { unk1 } => {
                format!("unk49 unk1={unk1}")
            }
            TfxBytecodeOp::Unk4a { unk1 } => {
                format!("unk4a unk1={unk1}")
            }
            TfxBytecodeOp::Unk4b { unk1 } => {
                format!("unk4b unk1={unk1}")
            }
            TfxBytecodeOp::Unk4c { unk1 } => {
                format!("unk4c unk1={unk1}")
            }
            TfxBytecodeOp::Unk4d { unk1 } => {
                format!("unk4d unk1={unk1}")
            }
            TfxBytecodeOp::Unk4e {
                unk1,
                unk2,
                unk3,
                unk4,
            } => {
                format!("unk4e unk1={unk1} unk2={unk2} unk3={unk3} unk4={unk4}")
            }
            TfxBytecodeOp::Unk4f { unk1 } => {
                format!("unk4f unk1={unk1}")
            }
            TfxBytecodeOp::Unk50 { unk1 } => format!("unk50 unk1={unk1}"),
            TfxBytecodeOp::Unk51 => "unk51".to_string(),
            TfxBytecodeOp::Unk52 { unk1, unk2 } => format!("unk52 unk1={unk1} unk2={unk2}"),
            TfxBytecodeOp::Unk53 { unk1, unk2 } => format!("unk53 unk1={unk1} unk2={unk2}"),
            TfxBytecodeOp::Unk54 { unk1, unk2 } => format!("unk54 unk1={unk1} unk2={unk2}"),
            TfxBytecodeOp::Unk55 => "unk55".to_string(),
            TfxBytecodeOp::Unk56 => "unk56".to_string(),
            TfxBytecodeOp::Unk57 => "unk57".to_string(),
            TfxBytecodeOp::Unk58 => "unk58".to_string(),
        }
    }
}

fn decode_permute_param(param: u8) -> String {
    let s0 = (param >> 6) & 0b11;
    let s1 = (param >> 4) & 0b11;
    let s2 = (param >> 2) & 0b11;
    let s3 = param & 0b11;
    const DIMS: [char; 4] = ['x', 'y', 'z', 'w'];

    format!(
        ".{}{}{}{}",
        DIMS[s0 as usize], DIMS[s1 as usize], DIMS[s2 as usize], DIMS[s3 as usize]
    )
}
