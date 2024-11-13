use std::io::Cursor;

use alkahest_data::tfx::TfxShaderStage;
use binrw::{binread, BinReaderExt, Endian};
use glam::Vec4;

use crate::tfx::externs::TfxExtern;

#[rustfmt::skip]
#[binread]
#[derive(Debug)]
pub enum TfxBytecodeOp {
    // Basic math ops
    #[br(magic = 0x01_u8)] Add,
    #[br(magic = 0x02_u8)] Subtract,
    #[br(magic = 0x03_u8)] Multiply,
    #[br(magic = 0x04_u8)] Divide,
    #[br(magic = 0x05_u8)] Multiply2,
    #[br(magic = 0x06_u8)] Add2,
    #[br(magic = 0x07_u8)] IsZero,
    #[br(magic = 0x08_u8)] Min,
    #[br(magic = 0x09_u8)] Max,

    #[br(magic = 0x0a_u8)] LessThan,
    #[br(magic = 0x0b_u8)] Dot,
    #[br(magic = 0x0c_u8)] Merge1_3,
    #[br(magic = 0x0d_u8)] Merge2_2,
    #[br(magic = 0x0e_u8)] Merge3_1,
    #[br(magic = 0x0f_u8)] Cubic,
    #[br(magic = 0x10_u8)] Lerp,
    #[br(magic = 0x11_u8)] LerpSaturated,
    #[br(magic = 0x12_u8)] MultiplyAdd,
    #[br(magic = 0x13_u8)] Clamp,
    #[br(magic = 0x14_u8)] Unk14,
    #[br(magic = 0x15_u8)] Abs,
    #[br(magic = 0x16_u8)] Signum,
    #[br(magic = 0x17_u8)] Floor,
    #[br(magic = 0x18_u8)] Ceil,
    #[br(magic = 0x19_u8)] Round,
    #[br(magic = 0x1a_u8)] Frac,
    #[br(magic = 0x1b_u8)] Unk1b,
    #[br(magic = 0x1c_u8)] Unk1c,
    #[br(magic = 0x1d_u8)] Negate,
    #[br(magic = 0x1e_u8)] VectorRotationsSin, // _trig_helper_vector_sin_rotations_estimate
    #[br(magic = 0x1f_u8)] VectorRotationsCos, // _trig_helper_vector_cos_rotations_estimate
    #[br(magic = 0x20_u8)] VectorRotationsSinCos, // _trig_helper_vector_sin_cos_rotations_estimate
    #[br(magic = 0x21_u8)] PermuteExtendX, // Alias for permute(.xxxx)
    #[br(magic = 0x22_u8)] Permute { fields: u8 }, // Permute/swizzle values
    #[br(magic = 0x23_u8)] Saturate,
    #[br(magic = 0x24_u8)] Unk24,
    #[br(magic = 0x25_u8)] Unk25,
    #[br(magic = 0x26_u8)] Unk26,
    #[br(magic = 0x27_u8)] Triangle,
    #[br(magic = 0x28_u8)] Jitter,
    #[br(magic = 0x29_u8)] Wander,
    #[br(magic = 0x2a_u8)] Rand,
    #[br(magic = 0x2b_u8)] RandSmooth,
    #[br(magic = 0x2c_u8)] Unk2c,
    #[br(magic = 0x2d_u8)] Unk2d,
    #[br(magic = 0x2e_u8)] TransformVec4,

    // Constant-related
    #[br(magic = 0x34_u8)] PushConstVec4 { constant_index: u8 },
    #[br(magic = 0x35_u8)] LerpConstant { constant_start: u8 },
    #[br(magic = 0x36_u8)] LerpConstantSaturated { constant_start: u8 },
    #[br(magic = 0x37_u8)] Spline4Const { constant_start: u8 },
    #[br(magic = 0x38_u8)] Spline8Const { constant_start: u8 },
    #[br(magic = 0x39_u8)] Spline8ChainConst { constant_start: u8 },
    #[br(magic = 0x3a_u8)] Gradient4Const { constant_start: u8 },
    #[br(magic = 0x3b_u8)] Unk3b { constant_start: u8 },

    // Externs
    /// Pushes an extern float to the stack, extended to all 4 elements (value.xxxx)
    /// Offset is in single floats (4 bytes)
    #[br(magic = 0x3c_u8)] PushExternInputFloat { extern_: TfxExtern, offset: u8 },
    /// Pushes an extern vec4 to the stack
    /// Offset is in vec4s (16 bytes)
    #[br(magic = 0x3d_u8)] PushExternInputVec4 { extern_: TfxExtern, offset: u8 },
    /// Pushes an extern mat4 to the stack
    /// Offset is in vec4s (16 bytes)
    #[br(magic = 0x3e_u8)] PushExternInputMat4 { extern_: TfxExtern, offset: u8 },
    /// Pushes an extern texture to the stack as a u64
    /// Offset is in u64s (8 bytes)
    #[br(magic = 0x3f_u8)] PushExternInputTextureView { extern_: TfxExtern, offset: u8 },
    /// Pushes an extern u32 to the stack as a u64
    /// Offset is in u32s (4 bytes)
    #[br(magic = 0x40_u8)] PushExternInputU32 { extern_: TfxExtern, offset: u8 },
    /// Pushes an extern UAV to the stack as a u64
    /// Offset is in u64s (8 bytes)
    #[br(magic = 0x41_u8)] PushExternInputUav { extern_: TfxExtern, offset: u8 },

    // TODO(cohae): Loads a value from render context + 0x44a0
    #[br(magic = 0x42_u8)] Unk42,
    #[br(magic = 0x43_u8)] PushFromOutput { element: u8 },
    #[br(magic = 0x44_u8)] PopOutput { element: u8 },
    #[br(magic = 0x45_u8)] PopOutputMat4 { element: u8 },
    #[br(magic = 0x46_u8)] PushTemp { slot: u8 },
    #[br(magic = 0x47_u8)] PopTemp { slot: u8 },
    #[br(magic = 0x48_u8)] SetShaderTexture {
        value: u8,
        #[br(try_calc(TfxShaderStage::from_tfx_value(value)))]
        stage: TfxShaderStage,
        #[br(calc(value & 0x1f))]
        slot: u8
    },
    #[br(magic = 0x49_u8)] Unk49 { unk1: u8 },
    #[br(magic = 0x4a_u8)] SetShaderSampler {
        value: u8,
        #[br(try_calc(TfxShaderStage::from_tfx_value(value)))]
        stage: TfxShaderStage,
        #[br(calc(value & 0x1f))]
        slot: u8
    },
    #[br(magic = 0x4b_u8)] SetShaderUav {
        value: u8,
        #[br(try_calc(TfxShaderStage::from_tfx_value(value)))]
        stage: TfxShaderStage,
        #[br(calc(value & 0x1f))]
        slot: u8
    },
    #[br(magic = 0x4c_u8)] Unk4c { unk1: u8 },
    /// Pushes a sampler on the stack from the technique sampler table
    #[br(magic = 0x4d_u8)] PushSampler { index: u8 },

    #[br(magic = 0x4e_u8)] PushObjectChannelVector { #[br(big)] hash: u32 },
    #[br(magic = 0x4f_u8)] PushGlobalChannelVector { unk1: u8 },
    #[br(magic = 0x50_u8)] Unk50 { unk1: u8 },
    #[br(magic = 0x51_u8)] Unk51,
    #[br(magic = 0x52_u8)] PushTexDimensions { index: u8, fields: u8 },
    #[br(magic = 0x53_u8)] PushTexTilingParams { index: u8, fields: u8 },
    #[br(magic = 0x54_u8)] PushTexTileLayerCount { index: u8, fields: u8 },
    #[br(magic = 0x55_u8)] Unk55,
    #[br(magic = 0x56_u8)] Unk56,
    #[br(magic = 0x57_u8)] Unk57,
    #[br(magic = 0x58_u8)] Unk58,
}

impl TfxBytecodeOp {
    pub fn parse_all(data: &[u8], endian: Endian) -> anyhow::Result<Vec<TfxBytecodeOp>> {
        let mut cur = Cursor::new(data);
        let mut opcodes = vec![];

        while (cur.position() as usize) < data.len() {
            let op = cur.read_type::<TfxBytecodeOp>(endian)?;
            opcodes.push(op);
        }

        Ok(opcodes)
    }

    pub fn name(&self) -> &'static str {
        match self {
            TfxBytecodeOp::Add => "add",
            TfxBytecodeOp::Subtract => "subtract",
            TfxBytecodeOp::Multiply => "multiply",
            TfxBytecodeOp::Divide => "divide",
            TfxBytecodeOp::Multiply2 => "multiply2",
            TfxBytecodeOp::Add2 => "add2",
            TfxBytecodeOp::IsZero => "is_zero",
            TfxBytecodeOp::Min => "min",
            TfxBytecodeOp::Max => "max",
            TfxBytecodeOp::LessThan => "less_than",
            TfxBytecodeOp::Dot => "dot",
            TfxBytecodeOp::Merge1_3 => "merge_1_3",
            TfxBytecodeOp::Merge2_2 => "merge_2_2",
            TfxBytecodeOp::Merge3_1 => "merge_3_1",
            TfxBytecodeOp::Cubic => "cubic",
            TfxBytecodeOp::Lerp => "lerp",
            TfxBytecodeOp::LerpSaturated => "lerp_saturated",
            TfxBytecodeOp::MultiplyAdd => "multiply_add",
            TfxBytecodeOp::Clamp => "clamp",
            TfxBytecodeOp::Unk14 => "unk14",
            TfxBytecodeOp::Abs => "abs",
            TfxBytecodeOp::Signum => "signum",
            TfxBytecodeOp::Floor => "floor",
            TfxBytecodeOp::Ceil => "ceil",
            TfxBytecodeOp::Round => "round",
            TfxBytecodeOp::Frac => "frac",
            TfxBytecodeOp::Unk1b => "unk1b",
            TfxBytecodeOp::Unk1c => "unk1c",
            TfxBytecodeOp::Negate => "negate",
            TfxBytecodeOp::VectorRotationsSin => "vector_rotations_sin",
            TfxBytecodeOp::VectorRotationsCos => "vector_rotations_cos",
            TfxBytecodeOp::VectorRotationsSinCos => "vector_rotations_sin_cos",
            TfxBytecodeOp::PermuteExtendX => "permute(.xxxx) (permute_extend_x)",
            TfxBytecodeOp::Permute { .. } => "permute",
            TfxBytecodeOp::Saturate => "saturate",
            TfxBytecodeOp::Unk24 => "unk24",
            TfxBytecodeOp::Unk25 => "unk25",
            TfxBytecodeOp::Unk26 => "unk26",
            TfxBytecodeOp::Triangle => "triangle",
            TfxBytecodeOp::Jitter => "jitter",
            TfxBytecodeOp::Wander => "wander",
            TfxBytecodeOp::Rand => "rand",
            TfxBytecodeOp::RandSmooth => "rand_smooth",
            TfxBytecodeOp::Unk2c => "unk2c",
            TfxBytecodeOp::Unk2d => "unk2d",
            TfxBytecodeOp::TransformVec4 => "transform_vec4",
            TfxBytecodeOp::PushConstVec4 { .. } => "push_const_vec4",
            TfxBytecodeOp::LerpConstant { .. } => "lerp_constant",
            TfxBytecodeOp::LerpConstantSaturated { .. } => "lerp_constant_saturated",
            TfxBytecodeOp::Spline4Const { .. } => "spline4_const",
            TfxBytecodeOp::Spline8Const { .. } => "spline8_const",
            TfxBytecodeOp::Spline8ChainConst { .. } => "spline8_chain_const",
            TfxBytecodeOp::Gradient4Const { .. } => "gradient4_const",
            TfxBytecodeOp::Unk3b { .. } => "unk3b",
            TfxBytecodeOp::PushExternInputFloat { .. } => "push_extern_input_float",
            TfxBytecodeOp::PushExternInputVec4 { .. } => "push_extern_input_vec4",
            TfxBytecodeOp::PushExternInputMat4 { .. } => "push_extern_input_mat4",
            TfxBytecodeOp::PushExternInputTextureView { .. } => "push_extern_input_tex",
            TfxBytecodeOp::PushExternInputU32 { .. } => "push_extern_input_u32",
            TfxBytecodeOp::PushExternInputUav { .. } => "push_extern_input_uav",
            TfxBytecodeOp::Unk42 => "unk42",
            TfxBytecodeOp::PushFromOutput { .. } => "push_from_output",
            TfxBytecodeOp::PopOutput { .. } => "pop_output",
            TfxBytecodeOp::PopOutputMat4 { .. } => "pop_output_mat4",
            TfxBytecodeOp::PushTemp { .. } => "push_temp",
            TfxBytecodeOp::PopTemp { .. } => "pop_temp",
            TfxBytecodeOp::SetShaderTexture { .. } => "set_shader_texture",
            TfxBytecodeOp::Unk49 { .. } => "unk49",
            TfxBytecodeOp::SetShaderSampler { .. } => "set_shader_sampler",
            TfxBytecodeOp::SetShaderUav { .. } => "set_shader_uav",
            TfxBytecodeOp::Unk4c { .. } => "unk4c",
            TfxBytecodeOp::PushSampler { .. } => "push_sampler",
            TfxBytecodeOp::PushObjectChannelVector { .. } => "push_object_channel_vector",
            TfxBytecodeOp::PushGlobalChannelVector { .. } => "push_global_channel_vector",
            TfxBytecodeOp::Unk50 { .. } => "unk50",
            TfxBytecodeOp::Unk51 => "unk51",
            TfxBytecodeOp::PushTexDimensions { .. } => "push_tex_dimensions",
            TfxBytecodeOp::PushTexTilingParams { .. } => "push_tex_tiling_params",
            TfxBytecodeOp::PushTexTileLayerCount { .. } => "push_tex_tile_layer_count",
            TfxBytecodeOp::Unk55 => "unk55",
            TfxBytecodeOp::Unk56 => "unk56",
            TfxBytecodeOp::Unk57 => "unk57",
            TfxBytecodeOp::Unk58 => "unk58",
        }
    }

    /// Formats the opcode to assembly-like output
    pub fn disassemble(&self, constants: Option<&[Vec4]>) -> String {
        let name = self.name();
        match self {
            TfxBytecodeOp::Permute { fields } => {
                format!("permute({})", decode_permute_param(*fields))
            }
            TfxBytecodeOp::PushConstVec4 { constant_index } => {
                if let Some(constants) = constants {
                    format!(
                        "push_const_vec4({constant_index}) // {}",
                        constants
                            .get(*constant_index as usize)
                            .map(Vec4::to_string)
                            .unwrap_or("CONSTANT OUT OF RANGE".into())
                    )
                } else {
                    format!("push_const_vec4({constant_index})")
                }
            }
            TfxBytecodeOp::LerpConstant { constant_start } => {
                if let Some(constants) = constants {
                    format!(
                        "lerp_constant({}, {}) // a={} b={}",
                        constant_start,
                        constant_start + 1,
                        constants
                            .get(*constant_start as usize)
                            .map(Vec4::to_string)
                            .unwrap_or("CONSTANT OUT OF RANGE".into()),
                        constants
                            .get(*constant_start as usize + 1)
                            .map(Vec4::to_string)
                            .unwrap_or("CONSTANT OUT OF RANGE".into())
                    )
                } else {
                    format!("lerp_constant({}, {})", constant_start, constant_start + 1)
                }
            }
            TfxBytecodeOp::LerpConstantSaturated { constant_start } => {
                // not really used in regular bytecode
                format!(
                    "lerp_constant_saturated({}, {})",
                    constant_start,
                    constant_start + 1
                )
            }
            TfxBytecodeOp::Spline4Const {
                constant_start: unk1,
            } => {
                format!("spline4_const unk1={unk1}")
            }
            TfxBytecodeOp::Spline8Const { constant_start } => {
                format!("spline8_const constant_start={constant_start}")
            }
            TfxBytecodeOp::Spline8ChainConst { constant_start } => {
                format!("spline8_chain_const constant_start={constant_start}")
            }
            TfxBytecodeOp::Gradient4Const { constant_start } => {
                format!("gradient4_const constant_start={constant_start}")
            }
            TfxBytecodeOp::Unk3b { constant_start } => {
                if let Some(constants) = constants {
                    format!(
                        "unk_load_constant constants[{constant_start}] // {}",
                        constants
                            .get(*constant_start as usize)
                            .map(Vec4::to_string)
                            .unwrap_or("CONSTANT OUT OF RANGE".into())
                    )
                } else {
                    format!("unk_load_constant constants[{constant_start}]")
                }
            }
            TfxBytecodeOp::PushExternInputFloat { extern_, offset } => {
                format!(
                    "push_extern_input_float ({extern_:?}+0x{:X})",
                    (*offset as u32) * 4
                )
            }
            TfxBytecodeOp::PushExternInputVec4 { extern_, offset } => {
                format!(
                    "push_extern_input_vec4 ({extern_:?}+0x{:X})",
                    (*offset as u32) * 16
                )
            }
            TfxBytecodeOp::PushExternInputMat4 { extern_, offset } => {
                format!(
                    "push_extern_input_mat4 ({extern_:?}+0x{:X})",
                    (*offset as u32) * 16
                )
            }
            TfxBytecodeOp::PushExternInputTextureView { extern_, offset } => {
                format!(
                    "push_extern_input_tex ({extern_:?}+0x{:X})",
                    (*offset as u32) * 8
                )
            }
            TfxBytecodeOp::PushExternInputU32 { extern_, offset } => {
                format!(
                    "push_extern_input_u32 ({extern_:?}+0x{:X})",
                    (*offset as u32) * 4
                )
            }
            TfxBytecodeOp::PushExternInputUav { extern_, offset } => {
                format!(
                    "push_extern_input_uav ({extern_:?}+0x{:X})",
                    (*offset as u32) * 8
                )
            }
            TfxBytecodeOp::Unk42 => "unk42".to_string(),
            TfxBytecodeOp::PushFromOutput { element } => {
                format!("push_from_output({element})")
            }
            TfxBytecodeOp::PopOutput { element } => {
                format!("pop_output({element})")
            }
            TfxBytecodeOp::PopOutputMat4 { element } => {
                format!("pop_output_mat4({element})")
            }
            TfxBytecodeOp::PushTemp { slot } => {
                format!("push_temp({slot})")
            }
            TfxBytecodeOp::PopTemp { slot } => {
                format!("pop_temp({slot})")
            }
            TfxBytecodeOp::SetShaderTexture { stage, slot, .. } => {
                format!("set_shader_texture stage={stage:?} slot={slot}")
            }
            TfxBytecodeOp::Unk49 { unk1 } => {
                format!("unk49 unk1={unk1}")
            }
            TfxBytecodeOp::SetShaderSampler { stage, slot, .. } => {
                format!("set_shader_sampler stage={stage:?} slot={slot}")
            }
            TfxBytecodeOp::SetShaderUav { stage, slot, .. } => {
                format!("set_shader_uav stage={stage:?} slot={slot}")
            }
            TfxBytecodeOp::Unk4c { unk1 } => {
                format!("unk4c unk1={unk1}")
            }
            TfxBytecodeOp::PushSampler { index } => {
                format!("push_sampler index={index}")
            }
            TfxBytecodeOp::PushObjectChannelVector { hash } => {
                format!("push_object_channel_vector({hash:08X})")
            }
            TfxBytecodeOp::PushGlobalChannelVector { unk1: index } => {
                format!("push_global_channel_vector({index})")
            }
            TfxBytecodeOp::Unk50 { unk1 } => format!("unk50 unk1={unk1}"),
            TfxBytecodeOp::PushTexDimensions { index, fields } => format!(
                "push_tex_dimensions index={index} fields={}",
                decode_permute_param(*fields)
            ),
            TfxBytecodeOp::PushTexTilingParams { index, fields } => format!(
                "push_tex_tiling_params index={index} fields={}",
                decode_permute_param(*fields)
            ),
            TfxBytecodeOp::PushTexTileLayerCount { index, fields } => format!(
                "push_tex_tile_layer_count index={index} fields={}",
                decode_permute_param(*fields)
            ),
            _ => name.to_string(),
        }
    }

    /// Is this op PermuteExtendX or Permute(xxxx)?
    pub fn is_permute_x(&self) -> bool {
        matches!(
            self,
            TfxBytecodeOp::PermuteExtendX
                | TfxBytecodeOp::Permute {
                    //swizzle xx yy zz ww
                    fields: 0b00_00_00_00
                }
        )
    }
}

pub fn decode_permute_param(param: u8) -> String {
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
