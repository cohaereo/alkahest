use int_enum::IntEnum;

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, IntEnum)]
pub enum Opcode {
    Add = 0x1,
    Subtract,
    Multiply,
    Divide,
    Multiply_,
    Add_,
    IsZero,
    Min,
    Max,
    LessThan,
    Dot,
    Merge1_3,
    Merge2_2,
    Merge3_1,

    Cubic = 0x0F,
    Unknown0x10,
    Unknown0x11,
    Unknown0x12,
    Lerp,
    LerpSaturated,

    MultiplyAdd = 0x15,
    Clamp,
    Unknown0x17,
    Abs,
    Signum,
    Floor,
    Ceil,
    Round,
    Frac,
    Unknown0x1E,
    Unknown0x1F,
    Negate,
    VectorRotationsSin,
    VectorRotationsCos,
    VectorRotationsSinCos,

    Splat = 0x28,
    Permute,
    Saturate,
    Unknown0x24,
    Unknown0x25,
    Unknown0x26,
    Triangle,
    Jitter,
    Wander,
    Rand,
    RandSmooth,
    Unknown0x2C,
    Unknown0x2D,
    TransformVec4,

    CompareLessThan = 0x3b,
    CompareLessEqual,
    CompareGreaterThan,
    CompareGreaterEqual,
    CompareEqual,
    CompareNotEqual,
    CompareNotZeroTernary,

    PushConstVec4 = 0x42,
    LerpConstant,
    LerpConstantSaturated,

    Spline4Const = 0x45,
    Spline8Const,
    Spline8ChainConst,
    Gradient4Const,
    Unknown0x49,

    Unknown0x4A = 0x4a,
    PopOutput = 0x4c,

    // Extended instruction set (only used internally by the interpreter)
    ExtReturn = 0x80,
}

impl Opcode {
    /// Returns the size of the opcode in bytes, including the opcode itself.
    pub fn size(&self) -> usize {
        match self {
            Opcode::Add
            | Opcode::Add_
            | Opcode::Subtract
            | Opcode::Multiply
            | Opcode::Multiply_
            | Opcode::Divide
            | Opcode::IsZero
            | Opcode::Min
            | Opcode::Max
            | Opcode::LessThan
            | Opcode::Dot
            | Opcode::Merge1_3
            | Opcode::Merge2_2
            | Opcode::Merge3_1
            | Opcode::Cubic
            | Opcode::Unknown0x10
            | Opcode::Unknown0x11
            | Opcode::Unknown0x12
            | Opcode::Lerp
            | Opcode::LerpSaturated
            | Opcode::MultiplyAdd
            | Opcode::Clamp
            | Opcode::Unknown0x17
            | Opcode::Abs
            | Opcode::Signum
            | Opcode::Floor
            | Opcode::Ceil
            | Opcode::Round
            | Opcode::Frac
            | Opcode::Unknown0x1E
            | Opcode::Unknown0x1F
            | Opcode::Negate
            | Opcode::VectorRotationsSin
            | Opcode::VectorRotationsCos
            | Opcode::VectorRotationsSinCos
            | Opcode::Splat
            | Opcode::Saturate
            | Opcode::Triangle
            | Opcode::Jitter
            | Opcode::Wander
            | Opcode::Rand
            | Opcode::RandSmooth
            | Opcode::Unknown0x25
            | Opcode::Unknown0x26
            | Opcode::TransformVec4
            | Opcode::Unknown0x24
            | Opcode::Unknown0x2C
            | Opcode::Unknown0x2D
            | Opcode::CompareLessThan
            | Opcode::CompareLessEqual
            | Opcode::CompareGreaterThan
            | Opcode::CompareGreaterEqual
            | Opcode::CompareEqual
            | Opcode::CompareNotEqual
            | Opcode::CompareNotZeroTernary => 1,

            Opcode::PopOutput
            | Opcode::PushConstVec4
            | Opcode::LerpConstant
            | Opcode::Spline8Const
            | Opcode::Permute
            | Opcode::LerpConstantSaturated
            | Opcode::Spline4Const
            | Opcode::Spline8ChainConst
            | Opcode::Gradient4Const => 2,

            Opcode::ExtReturn => 1,

            // Unknowns
            Opcode::Unknown0x49 | Opcode::Unknown0x4A => 2,
        }
    }
}
pub fn disassemble(bytes: &[u8]) -> anyhow::Result<Vec<String>> {
    let mut result = Vec::new();

    let mut offset = 0;
    while offset < bytes.len() {
        let ptr = &bytes[offset..];
        let Ok(opcode) = Opcode::try_from(ptr[0]) else {
            return Err(anyhow::anyhow!(
                "Unknown opcode 0x{:02X} (remaining bytes {:02X?})",
                ptr[0],
                ptr
            ));
        };
        result.push(format!(
            "{:02X}: {:?} {:02X?}",
            offset,
            opcode,
            &ptr[1..opcode.size()]
        ));
        offset += opcode.size();
    }

    Ok(result)
}
