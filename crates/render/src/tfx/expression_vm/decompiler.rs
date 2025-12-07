use alkahest_data::tfx::{ExternIndex, ShaderStage};
use anyhow::{ensure, Context};
use glam::Vec4;

use super::opcodes::Opcode;
use crate::tfx::externs::Externs;

#[derive(Default, Debug)]
pub struct DecompilationResult {
    pub textures: Vec<(usize, ShaderStage, String)>,
    pub samplers: Vec<(usize, ShaderStage, String)>,
    pub uavs: Vec<(usize, ShaderStage, String)>,
    pub cb_expressions: Vec<(usize, String, bool)>,
}

impl DecompilationResult {
    fn process_all_tags(&mut self, strip: bool) {
        for (_slot, _stage, expr) in &mut self.samplers {
            *expr = Self::process_ansi_tags(expr, strip);
        }
        for (_slot, _stage, expr) in &mut self.textures {
            *expr = Self::process_ansi_tags(expr, strip);
        }
        for (_slot, _stage, expr) in &mut self.uavs {
            *expr = Self::process_ansi_tags(expr, strip);
        }
        for (_slot, expr, _is_float4x4) in &mut self.cb_expressions {
            *expr = Self::process_ansi_tags(expr, strip);
        }
    }

    fn process_ansi_tags(s: &str, strip: bool) -> String {
        const RESET: &str = "\x1b[0m";
        const BLUE: &str = "\x1b[34m";
        const YELLOW: &str = "\x1b[38;2;250;200;100m";
        const ORANGE: &str = "\x1b[38;2;230;144;100m";
        const GREEN: &str = "\x1b[32m";
        // rgb 200 130 255
        const MAGENTA: &str = "\x1b[38;2;200;130;255m";
        const CYAN: &str = "\x1b[38;2;90;200;255m";
        let tags = [
            ("<fun>", YELLOW),
            ("<op>", ORANGE),
            ("<num>", MAGENTA),
            ("<ident>", CYAN),
            ("<reset>", RESET),
        ];

        let mut r = s.to_string();
        for (tag, ansi) in &tags {
            if strip {
                r = r.replace(tag, "");
            } else {
                r = r.replace(tag, ansi);
            }
        }

        let ops = ["->", "+", " - ", "*", "/", "=", "<", ">"];

        for op in &ops {
            if !strip {
                r = r.replace(op, &format!("{}{}{}", ORANGE, op, RESET));
            }
        }

        r
    }

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
            for (slot, expr, is_float4x4) in &self.cb_expressions {
                let slot_fixed = if *is_float4x4 {
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

pub struct DecompilerState<'a> {
    data: &'a [u8],
    pub ip: usize,

    stack: [String; 32],
    stack_pointer: usize,

    temp: [String; 16],

    // Configuration
    use_ansi: bool,
}

impl<'a> DecompilerState<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            ip: 0,
            data,
            stack: std::array::from_fn(|_| "NULL".to_string()),
            stack_pointer: 0,
            temp: std::array::from_fn(|_| "NULL".to_string()),
            use_ansi: false,
        }
    }

    pub fn with_ansi(mut self, use_ansi: bool) -> Self {
        self.use_ansi = use_ansi;
        self
    }

    fn data_ptr(&self) -> &[u8] {
        &self.data[self.ip..]
    }

    #[must_use = "Pushed value must be stored in the cache register"]
    #[inline(always)]
    fn push(&mut self, value: String) -> anyhow::Result<String> {
        #[cfg(debug_assertions)]
        anyhow::ensure!(
            self.stack_pointer < self.stack.len(),
            "Stack overflow (ip=0x{:X}, sp={})",
            self.ip,
            self.stack_pointer
        );
        self.stack_pointer += 1;
        self.stack[self.stack_pointer] = value.clone();
        Ok(value)
    }

    // #[inline(always)]
    // fn pop(&mut self) -> anyhow::Result<Vec4> {
    //     anyhow::ensure!(self.stack_pointer < 16, "Stack underflow");
    //     let value = self.stack[self.stack_pointer];
    //     self.stack_pointer += 1;
    //     Ok(value)
    // }

    #[inline(always)]
    fn get(&self, index_relative: isize) -> anyhow::Result<String> {
        let index = self.stack_pointer as isize + index_relative;
        #[cfg(debug_assertions)]
        anyhow::ensure!(
            (0..16).contains(&index),
            "Stack index out of bounds (index={index}, ip=0x{:X})",
            self.ip
        );
        Ok(self.stack[index as usize].clone())
    }

    // Pops the top value off the stack and returns the value at the new top of the stack (or ZERO if the stack is empty)
    #[inline(always)]
    fn pop_top(&mut self) -> String {
        self.stack_pointer = self.stack_pointer.saturating_sub(1);
        self.stack
            .get(self.stack_pointer)
            .cloned()
            .unwrap_or("NULL".to_string())
    }

    #[inline(always)]
    fn stack_top(&mut self) -> &mut String {
        &mut self.stack[self.stack_pointer]
    }

    // // Pops the top N values off the stack and returns the value at the new top of the stack (or ZERO if the stack is empty)
    // fn pop_n(&mut self, n: usize) -> Vec4 {
    //     self.stack_pointer = self.stack_pointer.saturating_add(n);
    //     self.stack
    //         .get(self.stack_pointer)
    //         .copied()
    //         .unwrap_or(Vec4::ZERO)
    // }

    // #[profiling::function]
    pub fn evaluate(&mut self, constants: &[Vec4]) -> anyhow::Result<DecompilationResult> {
        let mut result = DecompilationResult::default();

        let mut cached_top = "NULL".to_string();

        macro_rules! set_top {
            ($value:expr) => {{
                cached_top = $value;
                *self.stack_top() = cached_top.clone();
            }};
        }

        let get_constant = |index: usize| -> anyhow::Result<String> {
            anyhow::ensure!(index < constants.len(), "Invalid constant index");
            let c = constants[index];
            Ok(format!(
                "<fun>float4<reset>(<num>{}<reset>, <num>{}<reset>, <num>{}<reset>, \
                 <num>{}<reset>)",
                c.x, c.y, c.z, c.w
            ))
        };
        let get_constants = |range: std::ops::Range<usize>| -> anyhow::Result<Vec<String>> {
            let mut r = Vec::new();
            for i in range {
                r.push(get_constant(i)?);
            }
            Ok(r)
        };

        'exec: while self.ip < self.data.len() {
            let ptr = self.data_ptr();
            let Ok(op) = Opcode::try_from(ptr[0]) else {
                anyhow::bail!("Invalid opcode: 0x{:02X} @ ip 0x{:X}", ptr[0], self.ip);
            };

            match op {
                Opcode::ExtReturn => {
                    break 'exec;
                }
                Opcode::Add | Opcode::Add_ => {
                    cached_top = format!("({} + {cached_top})", self.get(-1)?);
                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top.clone();
                }
                Opcode::Subtract => {
                    cached_top = format!("({} - {cached_top})", self.get(-1)?);
                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top.clone();
                }
                Opcode::Multiply | Opcode::Multiply_ => {
                    cached_top = format!("({} * {cached_top})", self.get(-1)?);
                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top.clone();
                }
                Opcode::Divide => {
                    let v1 = self.get(-1)?;

                    cached_top = format!("({v1} / {cached_top})");

                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top.clone();
                }
                Opcode::Min => {
                    cached_top = format!("<fun>min<reset>({}, {cached_top})", self.get(-1)?);
                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top.clone();
                }
                Opcode::Max => {
                    cached_top = format!("<fun>max<reset>({}, {cached_top})", self.get(-1)?);
                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top.clone();
                }
                Opcode::Dot => {
                    cached_top = format!("<fun>dot<reset>({}, {cached_top})", self.get(-1)?);
                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top.clone();
                }
                Opcode::Merge1_3 => {
                    let a0 = cached_top;
                    let a1 = self.get(-1)?;
                    self.stack_pointer -= 1;
                    cached_top = format!("<fun>float4<reset>({a1}.x, {a0}.xyz)");
                    *self.stack_top() = cached_top.clone();
                }
                Opcode::Merge2_2 => {
                    let a0 = cached_top;
                    let a1 = self.get(-1)?;
                    self.stack_pointer -= 1;
                    cached_top = format!("<fun>float4<reset>({a1}.xy, {a0}.xy)");
                    *self.stack_top() = cached_top.clone();
                }
                Opcode::Merge3_1 => {
                    let a0 = cached_top;
                    let a1 = self.get(-1)?;
                    self.stack_pointer -= 1;
                    set_top!(format!("<fun>float4<reset>({a1}.xyz, {a0}.x)"));
                }
                Opcode::Cubic => {
                    let x = cached_top;
                    let coefficients = self.get(-1)?;
                    set_top!(format!("<fun>cubic<reset>({x}, {coefficients})"))
                }
                Opcode::Lerp => {
                    let s = cached_top;
                    let y = self.get(-1)?;
                    let x = self.get(-2)?;
                    self.stack_pointer -= 2;
                    set_top!(format!("<fun>lerp<reset>({x}, {y}, {s})"));
                }
                Opcode::LerpConstant => {
                    let constant_start = ptr[1];
                    ensure!(
                        (constant_start + 1) < constants.len() as u8,
                        "Invalid constant index"
                    );
                    let a = get_constant(constant_start as usize)?;
                    let b = get_constant((constant_start + 1) as usize)?;
                    let t = cached_top;

                    cached_top = format!("<fun>lerp<reset>({a}, {b}, {t})");
                    *self.stack_top() = cached_top.clone();
                }
                Opcode::Spline4Const => {
                    let constant_start = ptr[1];
                    ensure!(
                        (constant_start + 4) < constants.len() as u8,
                        "Invalid constant index"
                    );

                    let cl = get_constants(constant_start as usize..constant_start as usize + 5)?;
                    cached_top = format!(
                        "<fun>spline4<reset>({}, {}, {}, {}, {}, {})",
                        cached_top, cl[0], cl[1], cl[2], cl[3], cl[4]
                    );
                }
                Opcode::Spline8Const => {
                    let constant_start = ptr[1];
                    ensure!(
                        (constant_start + 9) < constants.len() as u8,
                        "Invalid constant index"
                    );

                    let cl = get_constants(constant_start as usize..constant_start as usize + 10)?;
                    cached_top = format!(
                        "<fun>spline8<reset>({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {})",
                        cached_top,
                        cl[0],
                        cl[1],
                        cl[2],
                        cl[3],
                        cl[4],
                        cl[5],
                        cl[6],
                        cl[7],
                        cl[8],
                        cl[9]
                    );
                }
                Opcode::Gradient4Const => {
                    let constant_start = ptr[1];
                    ensure!(
                        (constant_start + 5) < constants.len() as u8,
                        "Invalid constant index"
                    );

                    let cl = get_constants(constant_start as usize..constant_start as usize + 60)?;
                    // cached_top = super::helpers::bytecode_op_gradient4_const(
                    //     cached_top, cl[0], cl[1], cl[2], cl[3], cl[4], cl[5],
                    // );
                    cached_top = format!(
                        "<fun>gradient4<reset>({}, {}, {}, {}, {}, {}, {})",
                        cached_top, cl[0], cl[1], cl[2], cl[3], cl[4], cl[5]
                    );
                }
                Opcode::MultiplyAdd => {
                    let c = cached_top;
                    let b = self.get(-1)?;
                    let a = self.get(-2)?;
                    self.stack_pointer -= 2;
                    set_top!(format!("({a} <op>*<op/> {b} <op>+<op/> {c})"));
                }
                Opcode::Clamp => {
                    let min = cached_top;
                    let max = self.get(-1)?;
                    let value = self.get(-2)?;
                    self.stack_pointer -= 2;
                    set_top!(format!("<fun>clamp<reset>({value}, {min}, {max})"));
                }
                Opcode::Floor => {
                    set_top!(format!("<fun>floor<reset>({cached_top})"));
                }
                Opcode::Ceil => {
                    set_top!(format!("<fun>ceil<reset>({cached_top})"));
                }
                Opcode::Round => {
                    set_top!(format!("<fun>round<reset>({cached_top})"));
                }
                Opcode::Frac => {
                    set_top!(format!("<fun>fract<reset>({cached_top})"));
                }
                Opcode::Unknown0x1F => {
                    set_top!(format!("<fun>bytecode_op_1f<reset>({cached_top})"));
                }
                Opcode::Negate => {
                    set_top!(format!("(-{cached_top})"));
                }
                Opcode::Splat => {
                    set_top!(format!("{cached_top}.xxxx"));
                }
                Opcode::Permute => {
                    let fields = ptr[1];
                    let x = (fields >> 6) & 0b11;
                    let y = (fields >> 4) & 0b11;
                    let z = (fields >> 2) & 0b11;
                    let w = fields & 0b11;

                    const COMPONENTS: [char; 4] = ['x', 'y', 'z', 'w'];

                    set_top!(format!(
                        "{cached_top}.{}{}{}{}",
                        COMPONENTS[x as usize],
                        COMPONENTS[y as usize],
                        COMPONENTS[z as usize],
                        COMPONENTS[w as usize]
                    ));
                }
                Opcode::Saturate => {
                    set_top!(format!("<fun>saturate<reset>({cached_top})"));
                }
                Opcode::Unknown0x25 => {
                    set_top!(format!("<fun>bytecode_op_25<reset>({cached_top})"));
                }
                Opcode::Triangle => {
                    set_top!(format!("<fun>triangle<reset>({cached_top})"));
                }
                Opcode::Jitter => {
                    set_top!(format!("<fun>jitter<reset>({cached_top})"));
                }
                Opcode::Wander => {
                    set_top!(format!("<fun>wander<reset>({cached_top})"));
                }
                Opcode::Rand => {
                    set_top!(format!("<fun>rand<reset>({cached_top})"));
                }
                Opcode::TransformVec4 => {
                    let value = cached_top;
                    let mat = self.get(-1)?;

                    self.stack_pointer -= 1;

                    set_top!(format!("<fun>mul<reset>({mat}, {value})"));
                }
                Opcode::VectorRotationsSin => {
                    // set_top!(super::helpers::_trig_helper_vector_sin_rotations_estimate(
                    //     cached_top
                    // ));
                    set_top!(format!(
                        "<fun>vector_sin_rotations_estimate<reset>({cached_top})"
                    ));
                }
                Opcode::VectorRotationsCos => {
                    set_top!(format!(
                        "<fun>vector_cos_rotations_estimate<reset>({cached_top})"
                    ));
                }
                Opcode::VectorRotationsSinCos => {
                    set_top!(format!(
                        "<fun>vector_sin_cos_rotations_estimate<reset>({cached_top})"
                    ));
                }
                Opcode::PushConstVec4 => {
                    let index = ptr[1];
                    anyhow::ensure!(index < constants.len() as u8, "Invalid constant index");
                    let c = get_constant(index as usize)?;
                    cached_top = self.push(c)?;
                }
                // Push a temporary value onto the stack
                Opcode::PushTemp => {
                    let slot = ptr[1];
                    anyhow::ensure!(slot < self.temp.len() as u8, "Invalid temp slot");
                    cached_top = self.push(self.temp[slot as usize].clone())?;
                }
                // Pop a temporary value from the stack and store it in the specified temp slot
                Opcode::PopTemp => {
                    let slot = ptr[1];
                    anyhow::ensure!(slot < self.temp.len() as u8, "Invalid temp slot");
                    self.temp[slot as usize] = cached_top;
                    cached_top = self.pop_top();
                }
                Opcode::PopTextureView => {
                    let shader_stage = ShaderStage::from_index(ptr[1] >> 5)
                        .context("Invalid shader stage value")?;
                    let slot = ptr[1] & 0x1F;
                    let v = cached_top;
                    cached_top = self.pop_top();
                    result.textures.push((slot as usize, shader_stage, v));
                }
                Opcode::PopSamplerState => {
                    let shader_stage = ShaderStage::from_index(ptr[1] >> 5)
                        .context("Invalid shader stage value")?;
                    let slot = ptr[1] & 0x1F;
                    let v = cached_top;
                    cached_top = self.pop_top();
                    result.samplers.push((slot as usize, shader_stage, v));
                }
                Opcode::PopUav => {
                    let shader_stage = ShaderStage::from_index(ptr[1] >> 5)
                        .context("Invalid shader stage value")?;
                    let slot = ptr[1] & 0x1F;
                    let v = cached_top;
                    cached_top = self.pop_top();
                    result.uavs.push((slot as usize, shader_stage, v));
                }
                Opcode::PushSamplerState => {
                    let index = ptr[1];

                    cached_top = self.push(format!("samplers[{index}]"))?;
                }
                Opcode::PushExternInputFloat => {
                    let extern_id = ExternIndex::try_from(ptr[1])
                        .ok()
                        .context("Invalid extern index")?;
                    let offset = ptr[2];

                    let field = Externs::get_extern_field_name(extern_id, offset as usize * 4)
                        .map(str::to_string)
                        .unwrap_or(format!("0x{:X}", offset as usize * 4));

                    cached_top = self.push(format!(
                        "<fun>extern<reset><float>(<ident>{extern_id:?}<reset>->{field})"
                    ))?;
                }
                Opcode::PushExternInputVec4 => {
                    let extern_id = ExternIndex::try_from(ptr[1])
                        .ok()
                        .context("Invalid extern index")?;
                    let offset = ptr[2];

                    let field = Externs::get_extern_field_name(extern_id, offset as usize * 16)
                        .map(str::to_string)
                        .unwrap_or(format!("0x{:X}", offset as usize * 16));
                    cached_top = self.push(format!(
                        "<fun>extern<reset><float4>(<ident>{extern_id:?}<reset>->{field})",
                    ))?;
                }
                Opcode::PushExternInputMat4 => {
                    let extern_id = ExternIndex::try_from(ptr[1])
                        .ok()
                        .context("Invalid extern index")?;
                    let offset = ptr[2];

                    let field = Externs::get_extern_field_name(extern_id, offset as usize * 16)
                        .map(str::to_string)
                        .unwrap_or(format!("0x{:X}", offset as usize * 16));
                    cached_top = self.push(format!(
                        "<fun>extern<reset><float4x4>(<ident>{extern_id:?}<reset>->{field})"
                    ))?;
                }
                Opcode::PushExternInputTextureView => {
                    let extern_id = ExternIndex::try_from(ptr[1])
                        .ok()
                        .context("Invalid extern index")?;
                    let offset = ptr[2];

                    let field = Externs::get_extern_field_name(extern_id, offset as usize * 8)
                        .map(str::to_string)
                        .unwrap_or(format!("0x{:X}", offset as usize * 8));
                    cached_top = self.push(format!(
                        "<fun>extern<reset><TextureView>(<ident>{extern_id:?}<reset>->{field})"
                    ))?;
                }
                Opcode::PushExternInputUav => {
                    let extern_id = ExternIndex::try_from(ptr[1])
                        .ok()
                        .context("Invalid extern index")?;
                    let offset = ptr[2];

                    let field = Externs::get_extern_field_name(extern_id, offset as usize * 8)
                        .map(str::to_string)
                        .unwrap_or(format!("0x{:X}", offset as usize * 8));
                    cached_top = self.push(format!(
                        "<fun>extern<reset><UAV>(<ident>{extern_id:?}<reset>->{field})"
                    ))?;
                }
                Opcode::PushFromOutput => {
                    let element = ptr[1] as usize;
                    cached_top = self.push(format!("output[{element}]"))?;
                }
                Opcode::PopOutput => {
                    let element = ptr[1] as usize;
                    let v = cached_top;
                    cached_top = self.pop_top();
                    result.cb_expressions.push((element, v, false));
                }
                Opcode::PopOutputMat4 => {
                    let start_element = ptr[1] as usize;
                    let v = cached_top;
                    cached_top = self.pop_top();
                    result.cb_expressions.push((start_element, v, true));
                }
                Opcode::PushObjectChannelVector => {
                    let channel = u32::from_be_bytes([ptr[1], ptr[2], ptr[3], ptr[4]]);
                    cached_top = self.push(format!(
                        "<ident>object_channels<reset>[<num>{channel:08X}<reset>]"
                    ))?;
                }
                Opcode::PushGlobalChannelVector => {
                    let channel = ptr[1];
                    cached_top = self.push(format!(
                        "<ident>global_channels<reset>[<num>{channel}<reset>]"
                    ))?;
                }
                Opcode::Unknown0x5e => {
                    let unk = ptr[1];
                    cached_top = self.push(format!("unknown0x5e({unk})"))?;
                }
                u => {
                    anyhow::bail!("Unimplemented opcode: {u:?} / 0x{:02X}", ptr[0]);
                }
            }

            self.ip += op.size();
        }

        result.process_all_tags(!self.use_ansi);
        Ok(result)
    }
}
