use alkahest_data::tfx::TfxShaderStage;
use glam::Vec4;
use smallvec::SmallVec;

use crate::tfx::{bytecode::opcodes::TfxBytecodeOp, externs::ExternStorage};

#[derive(Default, Debug)]
pub struct DecompilationResult {
    pub textures: Vec<(usize, TfxShaderStage, String)>,
    pub samplers: Vec<(usize, TfxShaderStage, String)>,
    pub uavs: Vec<(usize, TfxShaderStage, String)>,
    pub cb_expressions: Vec<(usize, String)>,
}

impl DecompilationResult {
    pub fn pretty_print(&self) -> String {
        let mut r = String::new();

        if !self.samplers.is_empty() {
            r.push_str("// Samplers\n");
            for (slot, _stage, expr) in &self.samplers {
                r.push_str(&format!("SamplerState s{slot} = {expr};\n"));
            }
        }

        if !self.textures.is_empty() {
            r.push_str("\n// Textures\n");
            for (slot, _stage, expr) in &self.textures {
                r.push_str(&format!("Texture<float4> t{slot} = {expr};\n"));
            }
        }

        if !self.uavs.is_empty() {
            r.push_str("\n// UAVs\n");
            for (slot, _stage, expr) in &self.uavs {
                r.push_str(&format!("RWTexture<float4> t{slot} = {expr};\n"));
            }
        }

        if !self.cb_expressions.is_empty() {
            r.push_str("\n// Constant buffer\n");
            for (slot, expr) in &self.cb_expressions {
                let slot_fixed = if expr.starts_with("extern<float4x4>") {
                    format!("{slot}..={}", slot + 3)
                } else {
                    format!("{slot}")
                };
                r.push_str(&format!("cb0[{slot_fixed}] = {expr};\n"));
            }
        }

        r
    }
}

pub struct TfxBytecodeDecompiler;

impl TfxBytecodeDecompiler {
    pub fn decompile(
        opcodes: &[TfxBytecodeOp],
        constants: &[Vec4],
    ) -> anyhow::Result<DecompilationResult> {
        let mut r = DecompilationResult::default();

        let mut stack: SmallVec<[String; 64]> = Default::default();
        let mut temp: SmallVec<[String; 16]> = Default::default();
        for i in 0..temp.capacity() {
            temp.push(format!("TEMP_UNDEFINED_{i}"));
        }

        // TODO(cohae): Might need some cleanup
        macro_rules! stack_pop {
            () => {{
                anyhow::ensure!(!stack.is_empty());
                stack.pop().unwrap()
            }};
            (1) => {{
                [stack_pop!()]
            }};
            (2) => {{
                [stack_pop!(), stack_pop!()]
            }};
            (3) => {{
                [stack_pop!(), stack_pop!(), stack_pop!()]
            }};
        }

        macro_rules! stack_push {
            ($value:expr) => {{
                anyhow::ensure!(stack.len() < stack.capacity());
                stack.push($value);
            }};
        }

        macro_rules! stack_top {
            () => {{
                anyhow::ensure!(!stack.is_empty());
                stack.last_mut().unwrap()
            }};
        }

        for op in opcodes.iter() {
            match op {
                TfxBytecodeOp::Add | TfxBytecodeOp::Add2 => {
                    let [t1, t2] = stack_pop!(2);
                    stack_push!(format!("({t1} + {t2})"));
                }
                TfxBytecodeOp::Subtract => {
                    let [t1, t2] = stack_pop!(2);
                    stack_push!(format!("({t1} - {t2})"));
                }
                TfxBytecodeOp::Multiply | TfxBytecodeOp::Multiply2 => {
                    let [t1, t2] = stack_pop!(2);
                    stack_push!(format!("({t1} * {t2})"));
                }
                TfxBytecodeOp::Divide => {
                    let [t1, t2] = stack_pop!(2);
                    stack_push!(format!("({t1} / {t2})"));
                }
                TfxBytecodeOp::IsZero => {
                    let v = stack_pop!();
                    stack_push!(format!("({v} == 0)"));
                }
                TfxBytecodeOp::Min => {
                    let [t1, t2] = stack_pop!(2);
                    stack_push!(format!("min({t1}, {t2})"));
                }
                TfxBytecodeOp::Max => {
                    let [t1, t2] = stack_pop!(2);
                    stack_push!(format!("max({t1}, {t2})"));
                }
                TfxBytecodeOp::LessThan => {
                    let [t1, t2] = stack_pop!(2);
                    stack_push!(format!("({t1} < {t2})"));
                }
                TfxBytecodeOp::Dot => {
                    let [t1, t2] = stack_pop!(2);
                    stack_push!(format!("dot({t1}, {t2})"));
                }
                TfxBytecodeOp::Merge1_3 => {
                    let [t1, t2] = stack_pop!(2);
                    stack_push!(format!("float4({t1}.x, {t2}.xyz)"));
                }
                TfxBytecodeOp::Merge2_2 => {
                    let [t1, t2] = stack_pop!(2);
                    stack_push!(format!("float4({t1}.xy, {t2}.xy)"));
                }
                TfxBytecodeOp::Merge3_1 => {
                    let [t1, t2] = stack_pop!(2);
                    stack_push!(format!("float4({t1}.xyz, {t2}.x)"));
                }
                TfxBytecodeOp::Unk0f => {
                    let [t1, t2] = stack_pop!(2);
                    stack_push!(format!("unk0f({t1}, {t2})"));
                }
                TfxBytecodeOp::Lerp => {
                    let [t1, t2, t3] = stack_pop!(3);
                    stack_push!(format!("lerp({t1}, {t2}, {t3})"));
                }
                TfxBytecodeOp::LerpSaturated => {
                    let [t1, t2, t3] = stack_pop!(3);
                    stack_push!(format!("saturate(lerp({t1}, {t2}, {t3}))"));
                }
                TfxBytecodeOp::MultiplyAdd => {
                    let [t1, t2, t3] = stack_pop!(3);
                    stack_push!(format!("fma({t1}, {t2}, {t3})"));
                }
                TfxBytecodeOp::Clamp => {
                    let [t1, t2, t3] = stack_pop!(3);
                    stack_push!(format!("clamp({t1}, {t2}, {t3})"));
                }
                // TfxBytecodeOp::Unk14 => todo!(),
                TfxBytecodeOp::Abs => {
                    let v = stack_pop!();
                    stack_push!(format!("abs({v})"));
                }
                TfxBytecodeOp::Signum => {
                    let v = stack_pop!();
                    stack_push!(format!("sign({v})"));
                }
                TfxBytecodeOp::Floor => {
                    let v = stack_pop!();
                    stack_push!(format!("floor({v})"));
                }
                TfxBytecodeOp::Ceil => {
                    let v = stack_pop!();
                    stack_push!(format!("ceil({v})"));
                }
                TfxBytecodeOp::Round => {
                    let v = stack_pop!();
                    stack_push!(format!("round({v})"));
                }
                TfxBytecodeOp::Frac => {
                    let v = stack_pop!();
                    stack_push!(format!("frac({v})"));
                }
                // TfxBytecodeOp::Unk1b => todo!(),
                TfxBytecodeOp::Unk1c => {
                    let v = stack_pop!();
                    stack_push!(format!("unk1c({v})"));
                }
                TfxBytecodeOp::Negate => {
                    let v = stack_pop!();
                    stack_push!(format!("-{v}"));
                }
                TfxBytecodeOp::VectorRotationsSin => {
                    let v = stack_pop!();
                    stack_push!(format!("_trig_helper_vector_sin_rotations_estimate({v})"));
                }
                TfxBytecodeOp::VectorRotationsCos => {
                    let v = stack_pop!();
                    stack_push!(format!("_trig_helper_vector_cos_rotations_estimate({v})"));
                }
                TfxBytecodeOp::VectorRotationsSinCos => {
                    let v = stack_pop!();
                    stack_push!(format!(
                        "_trig_helper_vector_sin_cos_rotations_estimate({v})"
                    ));
                }
                TfxBytecodeOp::PermuteExtendX => {
                    let v = stack_top!();
                    v.push_str(".xxxx");
                }
                TfxBytecodeOp::Permute { fields } => {
                    let s0 = (fields >> 6) & 0b11;
                    let s1 = (fields >> 4) & 0b11;
                    let s2 = (fields >> 2) & 0b11;
                    let s3 = fields & 0b11;

                    let v = stack_top!();
                    let elements = ['x', 'y', 'z', 'w'];
                    v.push_str(&format!(
                        ".{}{}{}{}",
                        elements[s0 as usize],
                        elements[s1 as usize],
                        elements[s2 as usize],
                        elements[s3 as usize]
                    ));
                }
                TfxBytecodeOp::Saturate => {
                    let v = stack_pop!();
                    stack_push!(format!("saturate({v})"));
                }
                TfxBytecodeOp::Unk24 => {
                    let v = stack_pop!();
                    stack_push!(format!("unk24({v})"));
                }
                TfxBytecodeOp::Unk25 => {
                    let v = stack_pop!();
                    stack_push!(format!("unk25({v})"));
                }
                TfxBytecodeOp::Unk26 => {
                    let v = stack_pop!();
                    stack_push!(format!("unk26({v})"));
                }
                TfxBytecodeOp::Triangle => {
                    let v = stack_pop!();
                    stack_push!(format!("bytecode_op_triangle({v})"));
                }
                TfxBytecodeOp::Jitter => {
                    let v = stack_pop!();
                    stack_push!(format!("bytecode_op_jitter({v})"));
                }
                TfxBytecodeOp::Wander => {
                    let v = stack_pop!();
                    stack_push!(format!("bytecode_op_wander({v})"));
                }
                TfxBytecodeOp::Rand => {
                    let v = stack_pop!();
                    stack_push!(format!("bytecode_op_bytecode_op_rand({v})"));
                }
                TfxBytecodeOp::RandSmooth => {
                    let v = stack_pop!();
                    stack_push!(format!("bytecode_op_rand_smooth({v})"));
                }
                // TfxBytecodeOp::Unk2c => todo!(),
                // TfxBytecodeOp::Unk2d => todo!(),
                TfxBytecodeOp::TransformVec4 => {
                    let [v, m] = stack_pop!(2);
                    stack_push!(format!("mul({v}, {m})"));
                }
                &TfxBytecodeOp::PushConstVec4 { constant_index } => {
                    anyhow::ensure!((constant_index as usize) < constants.len());
                    let v = constants[constant_index as usize];
                    stack_push!(format!("float4({}, {}, {}, {})", v.x, v.y, v.z, v.w));
                }
                &TfxBytecodeOp::LerpConstant { constant_start } => {
                    anyhow::ensure!((constant_start as usize + 1) < constants.len());
                    let v = stack_pop!();

                    let start = constants[constant_start as usize];
                    let end = constants[constant_start as usize + 1];

                    let start =
                        format!("float4({}, {}, {}, {})", start.x, start.y, start.z, start.w);
                    let end = format!("float4({}, {}, {}, {})", end.x, end.y, end.z, end.w);

                    stack_push!(format!("lerp({start}, {end}, {v})",));
                }
                &TfxBytecodeOp::LerpConstantSaturated { constant_start } => {
                    anyhow::ensure!((constant_start as usize + 1) < constants.len());
                    let v = stack_pop!();

                    let start = constants[constant_start as usize];
                    let end = constants[constant_start as usize + 1];

                    let start =
                        format!("float4({}, {}, {}, {})", start.x, start.y, start.z, start.w);
                    let end = format!("float4({}, {}, {}, {})", end.x, end.y, end.z, end.w);

                    stack_push!(format!("saturate(lerp({start}, {end}, {v}))",));
                }
                &TfxBytecodeOp::Spline4Const { constant_start } => {
                    anyhow::ensure!((constant_start as usize + 4) < constants.len());
                    let c0 = constants[constant_start as usize];
                    let c1 = constants[constant_start as usize + 1];
                    let c2 = constants[constant_start as usize + 2];
                    let c3 = constants[constant_start as usize + 3];

                    let c0 = format!("float4({}, {}, {}, {})", c0.x, c0.y, c0.z, c0.w);
                    let c1 = format!("float4({}, {}, {}, {})", c1.x, c1.y, c1.z, c1.w);
                    let c2 = format!("float4({}, {}, {}, {})", c2.x, c2.y, c2.z, c2.w);
                    let c3 = format!("float4({}, {}, {}, {})", c3.x, c3.y, c3.z, c3.w);

                    stack_push!(format!(
                        "spline4_const({c0}, {c1}, {c2}, {c3})",
                        c0 = c0,
                        c1 = c1,
                        c2 = c2,
                        c3 = c3
                    ));
                }
                &TfxBytecodeOp::Gradient4Const { constant_start } => {
                    anyhow::ensure!((constant_start as usize + 4) < constants.len());
                    let c0 = constants[constant_start as usize];
                    let c1 = constants[constant_start as usize + 1];
                    let c2 = constants[constant_start as usize + 2];
                    let c3 = constants[constant_start as usize + 3];
                    let c4 = constants[constant_start as usize + 4];
                    let c5 = constants[constant_start as usize + 5];

                    let c0 = format!("float4({}, {}, {}, {})", c0.x, c0.y, c0.z, c0.w);
                    let c1 = format!("float4({}, {}, {}, {})", c1.x, c1.y, c1.z, c1.w);
                    let c2 = format!("float4({}, {}, {}, {})", c2.x, c2.y, c2.z, c2.w);
                    let c3 = format!("float4({}, {}, {}, {})", c3.x, c3.y, c3.z, c3.w);
                    let c4 = format!("float4({}, {}, {}, {})", c4.x, c4.y, c4.z, c4.w);
                    let c5 = format!("float4({}, {}, {}, {})", c5.x, c5.y, c5.z, c5.w);

                    let v = stack_pop!();
                    stack_push!(format!(
                        "gradient4_const({v}, {c0}, {c1}, {c2}, {c3}, {c4}, {c5})"
                    ));
                }
                &TfxBytecodeOp::Spline8Const { constant_start } => {
                    // anyhow::ensure!((constant_start as usize + 4) < constants.len());
                    let v = stack_pop!();
                    stack_push!(format!(
                        "spline8_const({v}, /* TODO: parse spline8 constants */)"
                    ));
                }
                // TfxBytecodeOp::Unk39 { unk1 } => todo!(),
                // TfxBytecodeOp::Unk3a { unk1 } => todo!(),
                &TfxBytecodeOp::Unk3b { constant_start } => {
                    anyhow::ensure!((constant_start as usize + 10) < constants.len());
                    let v = stack_pop!();
                    stack_push!(format!("unk3b({v}, /* TODO: parse unk3b constants */)"));
                }
                &TfxBytecodeOp::PushExternInputFloat { extern_, offset } => {
                    let offset_bytes = offset as usize * 4;
                    let path = ExternStorage::get_field_path(extern_, offset_bytes)
                        .unwrap_or_else(|| format!("{extern_:?}->_0x{offset_bytes:x}"));
                    stack_push!(format!("extern<float>({path})"));
                }
                &TfxBytecodeOp::PushExternInputVec4 { extern_, offset } => {
                    let offset_bytes = offset as usize * 16;
                    let path = ExternStorage::get_field_path(extern_, offset_bytes)
                        .unwrap_or_else(|| format!("{extern_:?}->_0x{offset_bytes:x}"));
                    stack_push!(format!("extern<float4>({path})"));
                }
                &TfxBytecodeOp::PushExternInputMat4 { extern_, offset } => {
                    let offset_bytes = offset as usize * 16;
                    let path = ExternStorage::get_field_path(extern_, offset_bytes)
                        .unwrap_or_else(|| format!("{extern_:?}->_0x{offset_bytes:x}"));
                    stack_push!(format!("extern<float4x4>({path})"));
                }
                &TfxBytecodeOp::PushExternInputTextureView { extern_, offset } => {
                    let offset_bytes = offset as usize * 8;
                    let path = ExternStorage::get_field_path(extern_, offset_bytes)
                        .unwrap_or_else(|| format!("{extern_:?}->_0x{offset_bytes:x}"));
                    stack_push!(format!("extern<Texture>({path})"));
                }
                &TfxBytecodeOp::PushExternInputU32 { extern_, offset } => {
                    let offset_bytes = offset as usize * 4;
                    let path = ExternStorage::get_field_path(extern_, offset_bytes)
                        .unwrap_or_else(|| format!("{extern_:?}->_0x{offset_bytes:x}"));
                    stack_push!(format!("extern<u32>({path})"));
                }
                &TfxBytecodeOp::PushExternInputUav { extern_, offset } => {
                    let offset_bytes = offset as usize * 8;
                    let path = ExternStorage::get_field_path(extern_, offset_bytes)
                        .unwrap_or_else(|| format!("{extern_:?}->_0x{offset_bytes:x}"));
                    stack_push!(format!("extern<UAV>({path})"));
                }
                // TfxBytecodeOp::Unk42 => todo!(),
                TfxBytecodeOp::PushFromOutput { element } => {
                    stack_push!(format!("cb0[{element}]"));
                }
                &TfxBytecodeOp::PopOutput { element } => {
                    let v = stack_pop!();
                    r.cb_expressions.push((element as usize, v));
                }
                // TODO(cohae): We need a better way to represent matrices in output
                &TfxBytecodeOp::PopOutputMat4 { element } => {
                    let v = stack_pop!();
                    r.cb_expressions.push((element as usize, v));
                }
                &TfxBytecodeOp::PushTemp { slot } => {
                    let slotu = slot as usize;
                    anyhow::ensure!(slotu < temp.len(), "Temp slot is out of range");

                    stack_push!(temp[slotu].clone());
                }
                &TfxBytecodeOp::PopTemp { slot } => {
                    let slotu = slot as usize;
                    anyhow::ensure!(slotu < temp.len(), "Temp slot is out of range");

                    temp[slotu] = stack_pop!();
                }
                &TfxBytecodeOp::SetShaderTexture { stage, slot, .. } => {
                    let v = stack_pop!();
                    r.textures.push((slot as usize, stage, v));
                }
                TfxBytecodeOp::Unk49 { unk1 } => {
                    let v = stack_pop!();
                    stack_push!(format!("unk49({unk1}, {v})"));
                }
                &TfxBytecodeOp::SetShaderSampler { stage, slot, .. } => {
                    let v = stack_pop!();
                    r.samplers.push((slot as usize, stage, v));
                }
                &TfxBytecodeOp::SetShaderUav { stage, slot, .. } => {
                    let v = stack_pop!();
                    r.uavs.push((slot as usize, stage, v));
                }
                TfxBytecodeOp::Unk4c { unk1 } => {
                    stack_push!(format!("unk4c({unk1})"));
                }
                TfxBytecodeOp::PushSampler { index } => {
                    stack_push!(format!("get_sampler({index})"));
                }
                TfxBytecodeOp::PushObjectChannelVector { hash } => {
                    stack_push!(format!("object_channel({hash:08X})"));
                }
                TfxBytecodeOp::PushGlobalChannelVector { unk1 } => {
                    stack_push!(format!("global_channel({unk1})"));
                }
                // TfxBytecodeOp::Unk50 { unk1 } => todo!(),
                // TfxBytecodeOp::Unk51 => todo!(),
                TfxBytecodeOp::Unk52 { unk1, unk2 } => {
                    stack_push!(format!("unk52({unk1}, {unk2})"));
                }
                // TfxBytecodeOp::Unk53 { unk1, unk2 } => todo!(),
                // TfxBytecodeOp::Unk54 { unk1, unk2 } => todo!(),
                // TfxBytecodeOp::Unk55 => todo!(),
                // TfxBytecodeOp::Unk56 => todo!(),
                // TfxBytecodeOp::Unk57 => todo!(),
                // TfxBytecodeOp::Unk58 => todo!(),
                _ => anyhow::bail!("Unsupported opcode for decompilation: {op:?}"),
            }
        }

        Ok(r)
    }
}
