use core::f32;
use std::ops::{Add, Mul, Sub};

use alkahest_data::tfx::{ExternIndex, ShaderStage};
use anyhow::{Context, ensure};
use d3d11::SamplerState;
use glam::{Mat4, Vec4, Vec4Swizzles};

use super::opcodes::Opcode;
use crate::{
    Renderer,
    gpu::command_list::ContextExt,
    tfx::externs::{ExternAccessor, ExternAccessorExt, TextureView, Uav},
    util::math::Vec4Ext,
};

#[derive(Default)]
pub struct TempObjectChannels {
    pub position: Vec4,
}

pub struct InterpreterState<'a> {
    data: &'a [u8],
    pub ip: usize,
    object_channels: Option<&'a TempObjectChannels>,
    context: Option<&'a d3d11::DeviceContext>,
    externs: Option<&'a dyn ExternAccessor>,

    stack: [Vec4; 32],
    stack_pointer: usize,

    temp: [Vec4; 16],
    debug: bool,
}

impl<'a> InterpreterState<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            ip: 0,
            data,
            object_channels: None,
            context: None,
            externs: None,
            stack: [Vec4::ZERO; 32],
            stack_pointer: 0,
            temp: [Vec4::ZERO; 16],
            debug: false,
        }
    }

    pub fn with_object_channels(mut self, object_channels: &'a TempObjectChannels) -> Self {
        self.object_channels = Some(object_channels);
        self
    }

    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    pub fn with_externs(mut self, externs: &'a dyn ExternAccessor) -> Self {
        self.externs = Some(externs);
        self
    }

    pub fn with_d3d11_context(mut self, context: &'a d3d11::DeviceContext) -> Self {
        self.context = Some(context);
        self
    }

    fn data_ptr(&self) -> &[u8] {
        &self.data[self.ip..]
    }

    #[must_use = "Pushed value must be stored in the cache register"]
    #[inline(always)]
    fn push(&mut self, value: Vec4) -> anyhow::Result<Vec4> {
        #[cfg(debug_assertions)]
        anyhow::ensure!(
            self.stack_pointer < self.stack.len(),
            "Stack overflow (ip=0x{:X}, sp={})",
            self.ip,
            self.stack_pointer
        );
        self.stack_pointer += 1;
        self.stack[self.stack_pointer] = value;
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
    fn get(&self, index_relative: isize) -> anyhow::Result<Vec4> {
        let index = self.stack_pointer as isize + index_relative;
        #[cfg(debug_assertions)]
        anyhow::ensure!(
            (0..32).contains(&index),
            "Stack index out of bounds (index {index}, ip=0x{:X})",
            self.ip
        );
        Ok(self
            .stack
            .get(index as usize)
            .context("Stack index out of bounds")?
            .to_owned())
    }

    // Pops the top value off the stack and returns the value at the new top of the stack (or ZERO if the stack is empty)
    #[inline(always)]
    fn pop_top(&mut self) -> Vec4 {
        self.stack_pointer = self.stack_pointer.saturating_sub(1);
        self.stack
            .get(self.stack_pointer)
            .copied()
            .unwrap_or(Vec4::ZERO)
    }

    #[inline(always)]
    fn stack_top(&mut self) -> &mut Vec4 {
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

    #[profiling::function]
    pub fn evaluate(
        &mut self,
        constants: &[Vec4],
        samplers: &[Option<SamplerState>],
        out: &mut [Vec4],
    ) -> anyhow::Result<()> {
        let mut cached_top = Vec4::ZERO;

        macro_rules! set_top {
            ($value:expr) => {{
                cached_top = $value;
                *self.stack_top() = cached_top;
            }};
        }

        'exec: while self.ip < self.data.len() {
            let ptr = self.data_ptr();
            let Ok(op) = Opcode::try_from(ptr[0]) else {
                anyhow::bail!("Invalid opcode: 0x{:02X} @ ip 0x{:X}", ptr[0], self.ip);
            };
            // profiling::scope!("evaluate_opcode", &format!("{op:?}"));

            match op {
                Opcode::ExtReturn => {
                    break 'exec;
                }
                Opcode::Add | Opcode::Add_ => {
                    cached_top += self.get(-1)?;
                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top;
                }
                Opcode::Subtract => {
                    cached_top = self.get(-1)? - cached_top;
                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top;
                }
                Opcode::Multiply | Opcode::Multiply_ => {
                    cached_top *= self.get(-1)?;
                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top;
                }
                Opcode::Divide => {
                    let v0 = cached_top;
                    let v1 = self.get(-1)?;
                    const EPSILON: Vec4 = Vec4::splat(1e-19);

                    let abs_v0 = v0.abs();
                    let v20 = abs_v0.cmpgt(EPSILON); // |v0| > epsilon

                    // Compute safe_part: signum(v1) * INFINITY
                    let safe_part = {
                        let sign = v1.signum();
                        sign * Vec4::INFINITY
                    };

                    // Select between actual division and safe_part based on v20
                    cached_top = Vec4::select(v20, v1 / v0, safe_part);

                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top;
                }

                Opcode::IsZero => {
                    let zero_mask = cached_top.cmpeq(Vec4::ZERO);
                    let result = Vec4::select(zero_mask, Vec4::ONE, Vec4::ZERO);
                    set_top!(result);
                }
                Opcode::Min => {
                    cached_top = cached_top.min(self.get(-1)?);
                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top;
                }
                Opcode::Max => {
                    cached_top = cached_top.max(self.get(-1)?);
                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top;
                }
                Opcode::LessThan => {
                    let cmp_mask = self.get(-1)?.cmplt(cached_top);
                    let result = Vec4::select(cmp_mask, Vec4::ONE, Vec4::ZERO);
                    self.stack_pointer -= 1;
                    set_top!(result);
                }
                Opcode::Dot => {
                    cached_top = Vec4::splat(self.get(-1)?.dot(cached_top));
                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top;
                }
                Opcode::Merge1_3 => {
                    let a0 = cached_top;
                    let a1 = self.get(-1)?;
                    self.stack_pointer -= 1;
                    cached_top = Vec4::new(a1.x, a0.x, a0.y, a0.z);
                    *self.stack_top() = cached_top;
                }
                Opcode::Merge2_2 => {
                    let a0 = cached_top;
                    let a1 = self.get(-1)?;
                    self.stack_pointer -= 1;
                    cached_top = Vec4::new(a1.x, a1.y, a0.x, a0.y);
                    *self.stack_top() = cached_top;
                }
                Opcode::Merge3_1 => {
                    let a0 = cached_top;
                    let a1 = self.get(-1)?;
                    self.stack_pointer -= 1;
                    set_top!(Vec4::new(a1.x, a1.y, a1.z, a0.x));
                }
                Opcode::Cubic => {
                    let x = cached_top;
                    let coefficients = self.get(-1)?;
                    self.stack_pointer -= 1;

                    let high = coefficients.x * x + coefficients.yyyy();
                    let low = coefficients.z * x + coefficients.wwww();
                    let x2 = x * x;

                    set_top!(high * x2 + low);
                }
                Opcode::Lerp => {
                    let s = cached_top;
                    let y = self.get(-1)?;
                    let x = self.get(-2)?;
                    self.stack_pointer -= 2;
                    set_top!((y - x) * s + x);
                }
                Opcode::LerpSaturated => {
                    let s = cached_top;
                    let y = self.get(-1)?;
                    let x = self.get(-2)?;
                    self.stack_pointer -= 2;
                    set_top!(((y - x) * s + x).clamp(Vec4::ZERO, Vec4::MAX));
                }
                Opcode::MultiplyAdd => {
                    #[cfg(target_feature = "fma")]
                    {
                        use arch::x86_64::{__m128, _mm_fmadd_ps};
                        let c: __m128 = cached_top.into();
                        let b: __m128 = self.get(-1)?.into();
                        let a: __m128 = self.get(-2)?.into();
                        self.stack_pointer -= 2;
                        set_top!(Vec4::from(unsafe { _mm_fmadd_ps(a, b, c) }));
                    }
                    #[cfg(not(target_feature = "fma"))]
                    {
                        let c = cached_top;
                        let b = self.get(-1)?;
                        let a = self.get(-2)?;
                        self.stack_pointer -= 2;
                        set_top!(a * b + c);
                    }
                }
                Opcode::Clamp => {
                    let min = cached_top;
                    let max = self.get(-1)?;
                    let value = self.get(-2)?;
                    self.stack_pointer -= 2;
                    set_top!(value.clamp(min, max));
                }
                Opcode::Abs => {
                    set_top!(cached_top.abs());
                }
                Opcode::Signum => {
                    set_top!(cached_top.signum());
                }
                Opcode::Floor => {
                    set_top!(cached_top.floor());
                }
                Opcode::Ceil => {
                    set_top!(cached_top.ceil());
                }
                Opcode::Round => {
                    set_top!(cached_top.round());
                }
                Opcode::Frac => {
                    set_top!(cached_top.fract());
                }
                Opcode::Unknown0x1F => {
                    let v58 = Vec4::mul(cached_top, cached_top);
                    let v59 = Vec4::add(Vec4::add(v58.yyyy(), v58.xxxx()), v58.zzzz());
                    let v60 = v59.rsqrt();
                    let v61 = Vec4::add(
                        Vec4::mul(
                            Vec4::sub(
                                Vec4::splat(0.5),
                                Vec4::mul(Vec4::mul(v60, v60), Vec4::mul(Vec4::splat(0.5), v59)),
                            ),
                            v60,
                        ),
                        v60,
                    );
                    let v62 = v61.cmpeq(v61);
                    let v63 = Vec4::select(v62, v61, v60);
                    let s = Vec4::select(Vec4::INFINITY.cmpeq(v63), Vec4::ZERO, cached_top * v63);
                    set_top!(s);
                }
                Opcode::Negate => {
                    set_top!(-cached_top);
                }
                Opcode::Splat => {
                    set_top!(cached_top.xxxx());
                }
                Opcode::Permute => {
                    let fields = ptr[1];
                    let x = (fields >> 6) & 0b11;
                    let y = (fields >> 4) & 0b11;
                    let z = (fields >> 2) & 0b11;
                    let w = fields & 0b11;

                    set_top!(Vec4::new(
                        cached_top[x as usize],
                        cached_top[y as usize],
                        cached_top[z as usize],
                        cached_top[w as usize],
                    ));
                }
                Opcode::Saturate => {
                    set_top!(cached_top.clamp(Vec4::ZERO, Vec4::ONE))
                }
                Opcode::Unknown0x25 => {
                    set_top!(super::helpers::bytecode_op_25(cached_top));
                }
                Opcode::Triangle => {
                    set_top!(super::helpers::bytecode_op_triangle(cached_top));
                }
                Opcode::Jitter => {
                    set_top!(super::helpers::bytecode_op_jitter(cached_top));
                }
                Opcode::Wander => {
                    set_top!(super::helpers::bytecode_op_wander(cached_top));
                }
                Opcode::Rand => {
                    set_top!(super::helpers::bytecode_op_rand(cached_top));
                }
                Opcode::TransformVec4 => {
                    let value = cached_top;
                    let w_axis = self.get(-1)?;
                    let z_axis = self.get(-2)?;
                    let y_axis = self.get(-3)?;
                    let x_axis = self.get(-4)?;

                    let mat = Mat4::from_cols(x_axis, y_axis, z_axis, w_axis);
                    self.stack_pointer -= 4;

                    set_top!(mat.mul_vec4(value));
                }
                Opcode::VectorRotationsSin => {
                    set_top!(super::helpers::_trig_helper_vector_sin_rotations_estimate(
                        cached_top
                    ));
                }
                Opcode::VectorRotationsCos => {
                    set_top!(super::helpers::_trig_helper_vector_cos_rotations_estimate(
                        cached_top
                    ));
                }
                Opcode::VectorRotationsSinCos => {
                    set_top!(
                        super::helpers::_trig_helper_vector_sin_cos_rotations_estimate(cached_top)
                    );
                }
                Opcode::PushConstVec4 => {
                    let index = ptr[1];
                    anyhow::ensure!(index < constants.len() as u8, "Invalid constant index");
                    cached_top = self.push(constants[index as usize])?;
                }
                Opcode::LerpConstant => {
                    let constant_start = ptr[1];
                    ensure!(
                        (constant_start + 1) < constants.len() as u8,
                        "Invalid constant index"
                    );
                    let a = constants[constant_start as usize];
                    let b = constants[(constant_start + 1) as usize];
                    let t = cached_top;

                    cached_top = a + t * (b - a);
                    *self.stack_top() = cached_top;
                }
                Opcode::Spline4Const => {
                    let constant_start = ptr[1];
                    ensure!(
                        (constant_start + 4) < constants.len() as u8,
                        "Invalid constant index"
                    );

                    let cl = &constants[constant_start as usize..constant_start as usize + 5];
                    cached_top = super::helpers::bytecode_op_spline4_const(
                        cached_top, cl[0], cl[1], cl[2], cl[3], cl[4],
                    );
                }
                Opcode::Spline8Const => {
                    let constant_start = ptr[1];
                    ensure!(
                        (constant_start + 9) < constants.len() as u8,
                        "Invalid constant index"
                    );

                    let cl = &constants[constant_start as usize..constant_start as usize + 10];
                    cached_top = super::helpers::bytecode_op_spline8_const(
                        cached_top, cl[0], cl[1], cl[2], cl[3], cl[4], cl[5], cl[6], cl[7], cl[8],
                        cl[9],
                    );
                }
                Opcode::Spline8ChainConst => {
                    let constant_start = ptr[1];
                    ensure!(
                        (constant_start + 9) < constants.len() as u8,
                        "Invalid constant index"
                    );

                    let x = cached_top;
                    let recursion = self.get(-1)?;
                    self.stack_pointer -= 1;

                    let cl = &constants[constant_start as usize..constant_start as usize + 10];
                    cached_top = super::helpers::bytecode_op_spline8_chain_const(
                        x, recursion, cl[0], cl[1], cl[2], cl[3], cl[4], cl[5], cl[6], cl[7],
                        cl[8], cl[9],
                    );
                }
                Opcode::Gradient4Const => {
                    let constant_start = ptr[1];
                    ensure!(
                        (constant_start + 5) < constants.len() as u8,
                        "Invalid constant index"
                    );

                    let cl = &constants[constant_start as usize..constant_start as usize + 6];
                    cached_top = super::helpers::bytecode_op_gradient4_const(
                        cached_top, cl[0], cl[1], cl[2], cl[3], cl[4], cl[5],
                    );
                }
                Opcode::Unknown0x49 => {
                    let constant_start = ptr[1];
                    ensure!(
                        (constant_start + 10) < constants.len() as u8,
                        "Invalid constant index"
                    );

                    let cl = &constants[constant_start as usize..];
                    set_top!(super::helpers::bytecode_op_unk3b_const(cached_top, cl));
                }
                // Push a temporary value onto the stack
                Opcode::PushTemp => {
                    let slot = ptr[1];
                    anyhow::ensure!(slot < self.temp.len() as u8, "Invalid temp slot");
                    cached_top = self.push(self.temp[slot as usize])?;
                }
                // Pop a temporary value from the stack and store it in the specified temp slot
                Opcode::PopTemp => {
                    let slot = ptr[1];
                    anyhow::ensure!(slot < self.temp.len() as u8, "Invalid temp slot");
                    self.temp[slot as usize] = cached_top;
                    cached_top = self.pop_top();
                }
                Opcode::PopTextureView => {
                    let Some(context) = self.context else {
                        anyhow::bail!("No D3D11 context set");
                    };
                    let Some(externs) = self.externs else {
                        anyhow::bail!("No externs set");
                    };

                    let shader_stage = ShaderStage::from_index(ptr[1] >> 5)
                        .context("Invalid shader stage value")?;
                    let slot = ptr[1] & 0x1F;
                    let bits = cached_top.x.to_bits();

                    let index = ExternIndex::try_from((bits >> 24) as u8)
                        .ok()
                        .context("Invalid extern index on pop texture view")?;
                    let offset = bits & 0xFFFFFF;

                    externs
                        .get_extern_value::<TextureView>(index, offset as usize)
                        .and_then(|o| {
                            let mut r = None;
                            o.get_srv(|srv| {
                                r = Some(());
                                context.set_shader_resource(shader_stage, slot as u32, srv);
                            });
                            r
                        })
                        .unwrap_or_else(|| {
                            Renderer::instance().get_extern_placeholder_texture(
                                index,
                                offset as usize,
                                |tex, _| {
                                    context.set_shader_resource(
                                        shader_stage,
                                        slot as u32,
                                        &tex.view,
                                    );
                                },
                            )
                        });
                }
                Opcode::PopSamplerState => {
                    let Some(context) = self.context else {
                        anyhow::bail!("No D3D11 context set");
                    };

                    let shader_stage = ShaderStage::from_index(ptr[1] >> 5)
                        .context("Invalid shader stage value")?;
                    let slot = ptr[1] & 0x1F;
                    let index = cached_top.x.to_bits();
                    cached_top = self.pop_top();
                    anyhow::ensure!(index < samplers.len() as u32, "Invalid sampler index");
                    let sampler = &samplers[index as usize];
                    context.set_sampler(shader_stage, slot as u32, sampler.as_ref());
                }
                Opcode::PopUav => {
                    let Some(context) = self.context else {
                        anyhow::bail!("No D3D11 context set");
                    };
                    let Some(externs) = self.externs else {
                        anyhow::bail!("No externs set");
                    };

                    let shader_stage = ShaderStage::from_index(ptr[1] >> 5)
                        .context("Invalid shader stage value")?;
                    if shader_stage != ShaderStage::Compute {
                        anyhow::bail!("Invalid shader stage for binding a UAV");
                    }
                    let slot = ptr[1] & 0x1F;
                    let bits = cached_top.x.to_bits();

                    let index = ExternIndex::try_from((bits >> 24) as u8)
                        .ok()
                        .context("Invalid extern index on pop texture view")?;
                    let offset = bits & 0xFFFFFF;

                    externs
                        .get_extern_value::<Uav>(index, offset as usize)
                        .and_then(|o| {
                            let mut r = None;
                            o.get_uav(|uav| {
                                r = Some(());
                                context.compute_set_unordered_access_views(
                                    slot as u32,
                                    &[Some(uav)],
                                    None,
                                );
                            });
                            r
                        })
                        .unwrap_or_else(|| {
                            Renderer::instance().get_extern_placeholder_texture(
                                index,
                                offset as usize,
                                |_, uav| {
                                    context.compute_set_unordered_access_views(
                                        slot as u32,
                                        &[Some(uav)],
                                        None,
                                    );
                                },
                            )
                        });
                }
                Opcode::PushSamplerState => {
                    let index = ptr[1];
                    anyhow::ensure!(index < samplers.len() as u8, "Invalid sampler index");
                    cached_top =
                        self.push(Vec4::new(f32::from_bits(index as u32), 0.0, 0.0, 0.0))?;
                }
                Opcode::PushExternInputFloat => {
                    let Some(externs) = self.externs else {
                        anyhow::bail!("No externs set");
                    };

                    let extern_id = ExternIndex::try_from(ptr[1])
                        .ok()
                        .context("Invalid extern index")?;
                    let offset = ptr[2];
                    let val = externs
                        .get_extern_value::<f32>(extern_id, offset as usize * 4)
                        .with_context(|| {
                            format!(
                                "Failed to get float extern value for {:?} @ 0x{:X}",
                                extern_id,
                                offset as usize * 4
                            )
                        })?;

                    cached_top = self.push(Vec4::splat(val))?;
                }
                Opcode::PushExternInputVec4 => {
                    let Some(externs) = self.externs else {
                        anyhow::bail!("No externs set");
                    };

                    let extern_id = ExternIndex::try_from(ptr[1])
                        .ok()
                        .context("Invalid extern index")?;
                    let offset = ptr[2];

                    let val = externs
                        .get_extern_value::<Vec4>(extern_id, offset as usize * 16)
                        .with_context(|| {
                            format!(
                                "Failed to get vec4 extern value for {:?} @ 0x{:X}",
                                extern_id,
                                offset as usize * 16
                            )
                        })?;

                    cached_top = self.push(val)?;
                }
                Opcode::PushExternInputMat4 => {
                    let Some(externs) = self.externs else {
                        anyhow::bail!("No externs set");
                    };

                    let extern_id = ExternIndex::try_from(ptr[1])
                        .ok()
                        .context("Invalid extern index")?;
                    let offset = ptr[2];

                    let val = externs
                        .get_extern_value::<Mat4>(extern_id, offset as usize * 16)
                        .with_context(|| {
                            format!(
                                "Failed to get mat4 extern value for {:?} @ 0x{:X}",
                                extern_id,
                                offset as usize * 16
                            )
                        })?;

                    self.push(val.x_axis)?;
                    self.push(val.y_axis)?;
                    self.push(val.z_axis)?;
                    cached_top = self.push(val.w_axis)?;
                }
                Opcode::PushExternInputTextureView => {
                    let extern_id = ExternIndex::try_from(ptr[1])
                        .ok()
                        .context("Invalid extern index")?;
                    let offset = ptr[2] as u32 * 8;

                    let bits = (extern_id as u32) << 24 | (offset & 0xFFFFFF);
                    cached_top = self.push(Vec4::new(f32::from_bits(bits), 0.0, 0.0, 0.0))?;
                }
                Opcode::PushExternInputUav => {
                    let extern_id = ExternIndex::try_from(ptr[1])
                        .ok()
                        .context("Invalid extern index")?;
                    let offset = ptr[2] as u32 * 8;

                    let bits = (extern_id as u32) << 24 | (offset & 0xFFFFFF);
                    cached_top = self.push(Vec4::new(f32::from_bits(bits), 0.0, 0.0, 0.0))?;
                }
                Opcode::PushFromOutput => {
                    let element = ptr[1] as usize;
                    anyhow::ensure!(element < out.len(), "Invalid output element index");
                    cached_top = self.push(out[element])?;
                }
                Opcode::PopOutput => {
                    let element = ptr[1] as usize;
                    anyhow::ensure!(element < out.len(), "Invalid output element index");
                    out[element] = cached_top;
                    cached_top = self.pop_top();
                }
                Opcode::PopOutputMat4 => {
                    let start_element = ptr[1] as usize;
                    anyhow::ensure!(
                        (start_element + 3) < out.len(),
                        "Invalid mat4 output starting element index"
                    );

                    let w_axis = cached_top;
                    let z_axis = self.get(-1)?;
                    let y_axis = self.get(-2)?;
                    let x_axis = self.get(-3)?;

                    out[start_element] = x_axis;
                    out[start_element + 1] = y_axis;
                    out[start_element + 2] = z_axis;
                    out[start_element + 3] = w_axis;

                    self.stack_pointer -= 4;
                    cached_top = self.get(0)?;
                }
                // Opcode::Unknown0x4D => {
                //     let channel = ptr[1];
                //     let val = match channel {
                //         0 => Vec4::ZERO, // Unsure, used as lerp paramter for warmind memories
                //         1 => self
                //             .object_channels
                //             .map(|c| c.position)
                //             .unwrap_or(Vec4::ZERO),
                //         _ => Vec4::ONE,
                //     };

                //     cached_top = self.push(val)?;
                // }
                Opcode::Unknown0x5e | Opcode::PushGlobalChannelVector => {
                    let channel = ptr[1];
                    // Direct indexing is safe here, as globals is 256 elements long
                    let val = Renderer::instance().externs.globals[channel as usize];
                    // let val = match channel {
                    //     124 => Vec4::X * 0.1,  // 138 in tfs
                    //     125 => Vec4::X * 1.0,  // 139 in tfs
                    //     128 => Vec4::X * 10.0, // ????
                    //     _ => Vec4::ONE,
                    // };
                    cached_top = self.push(val)?;
                }
                Opcode::PushObjectChannelVector => {
                    let channel_hash = u32::from_be_bytes([ptr[1], ptr[2], ptr[3], ptr[4]]);

                    let v = match channel_hash {
                        0xD3583E54 => Vec4::ZERO, // unique_id
                        _ => Vec4::ONE,
                    };

                    cached_top = self.push(v)?;
                }
                // TODO(cohae): This is a placeholder implementation
                Opcode::PushTexDimensions
                | Opcode::PushTexTilingParams
                | Opcode::PushTexTileLayerCount => {
                    let fields = ptr[1];
                    let s0 = (fields >> 6) & 0b11;
                    let s1 = (fields >> 4) & 0b11;
                    let s2 = (fields >> 2) & 0b11;
                    let s3 = fields & 0b11;

                    let v = Vec4::new(0.250000, 0.250000, 0.250000, 0.062500);
                    // let v = Vec4::new(0.200000, 0.164103, 0.200000, 0.033333);

                    let v2 = v.to_array();

                    cached_top = self.push(Vec4::new(
                        v2[s0 as usize],
                        v2[s1 as usize],
                        v2[s2 as usize],
                        v2[s3 as usize],
                    ))?;
                }
                u => {
                    anyhow::bail!(
                        "Unimplemented opcode: {u:?} / 0x{:02X} (ip=0x{:X})",
                        ptr[0],
                        self.ip
                    );
                }
            }

            if self.debug {
                println!("{}: {:?} {:?}", self.ip, op, &self.data_ptr()[1..op.size()]);
                // Print stack
                for (i, val) in self.stack.iter().enumerate() {
                    println!("  [{i}] {val:?}");
                }
            }

            self.ip += op.size();
        }

        Ok(())
    }
}
