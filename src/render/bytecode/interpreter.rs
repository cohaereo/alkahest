use std::{mem::transmute, ops::Neg};

use glam::{Mat4, Vec3, Vec4, Vec4Swizzles};
use tinyvec::ArrayVec;

use crate::render::{cbuffer::ConstantBufferCached, renderer::Renderer, RenderData};

use super::{
    externs::{TfxExtern, TfxShaderStage},
    opcodes::TfxBytecodeOp,
};

pub struct TfxBytecodeInterpreter {
    opcodes: Vec<TfxBytecodeOp>,
    pub error_shown: bool,
}

impl TfxBytecodeInterpreter {
    pub fn new(opcodes: Vec<TfxBytecodeOp>) -> Self {
        Self {
            opcodes,
            error_shown: false,
        }
    }

    pub fn evaluate(
        &self,
        renderer: &Renderer,
        render_data: &RenderData,
        buffer: &ConstantBufferCached<Vec4>,
        constants: &[Vec4],
    ) -> anyhow::Result<()> {
        let mut stack: ArrayVec<[Vec4; 64]> = Default::default();
        let mut temp = [Vec4::ZERO; 16];

        let buffer_map = buffer.data_array();

        macro_rules! stack_pop {
            ($pops:literal) => {{
                anyhow::ensure!(!stack.is_empty() && stack.len() >= $pops);
                let v: [Vec4; $pops] = stack[stack.len() - $pops..stack.len()].try_into().unwrap();
                stack.truncate(stack.len() - $pops);
                v
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

        for (ip, op) in self.opcodes.iter().enumerate() {
            match op {
                TfxBytecodeOp::Add | TfxBytecodeOp::Add2 => {
                    let [t1, t2] = stack_pop!(2);
                    stack_push!(t1 + t2);
                }
                TfxBytecodeOp::Subtract => {
                    let [t1, t0] = stack_pop!(2);
                    stack_push!(t1 - t0);
                }
                TfxBytecodeOp::Multiply | TfxBytecodeOp::Multiply2 => {
                    let [t1, t0] = stack_pop!(2);
                    stack_push!(t1 * t0);
                }
                TfxBytecodeOp::Divide => {
                    // cohae: Bungie's SIMD implementation appears to do some extra zero checks, don't know if those are necessary.
                    let [t1, t0] = stack_pop!(2);
                    stack_push!(t1 / t0);
                }
                TfxBytecodeOp::IsZero => {
                    // Cleaned up SIMD: _mm_and_ps(_mm_cmpeq_ps(a, _mm_setzero_ps()), _mm_set1_ps(1.0));
                    // Decompiled and simplified: value == 0.0 ? 1.0 : 0.0 (for each element in the vector)
                    let v = stack_top!();
                    let c = v.cmpeq(Vec4::ZERO);
                    *v = Vec4::new(
                        c.test(0).into(),
                        c.test(1).into(),
                        c.test(2).into(),
                        c.test(3).into(),
                    );
                }
                TfxBytecodeOp::LessThan => {
                    let [t1, t0] = stack_pop!(2);
                    let c = t0.cmplt(t1);
                    let v = Vec4::new(
                        c.test(0).into(),
                        c.test(1).into(),
                        c.test(2).into(),
                        c.test(3).into(),
                    );
                    stack_push!(v);
                }
                TfxBytecodeOp::Dot => {
                    let [t1, t0] = stack_pop!(2);
                    stack_push!(Vec4::splat(t0.dot(t1)));
                }
                TfxBytecodeOp::MultiplyAdd => {
                    let [t2, t1, t0] = stack_pop!(3);
                    stack_push!(t0 + (t1 * t2));
                }
                TfxBytecodeOp::Clamp => {
                    let [value, min, max] = stack_pop!(3);
                    stack_push!(value.clamp(min, max));
                }
                TfxBytecodeOp::Negate => {
                    let v = stack_top!();
                    *v = v.neg();
                }
                TfxBytecodeOp::Abs => {
                    let v = stack_top!();
                    *v = v.abs();
                }
                TfxBytecodeOp::Signum => {
                    let v = stack_top!();
                    *v = v.signum();
                }
                TfxBytecodeOp::Floor => {
                    let v = stack_top!();
                    *v = v.floor();
                }
                TfxBytecodeOp::Ceil => {
                    let v = stack_top!();
                    *v = v.ceil();
                }
                TfxBytecodeOp::Round => {
                    let v = stack_top!();
                    *v = v.round();
                }
                TfxBytecodeOp::VectorRotationsSin => {
                    let v = stack_top!();
                    *v = tfx_converted::_trig_helper_vector_sin_rotations_estimate(*v);
                }
                TfxBytecodeOp::VectorRotationsCos => {
                    let v = stack_top!();
                    *v = tfx_converted::_trig_helper_vector_cos_rotations_estimate(*v);
                }
                TfxBytecodeOp::VectorRotationsSinCos => {
                    let v = stack_top!();
                    *v = tfx_converted::_trig_helper_vector_sin_cos_rotations_estimate(*v);
                }
                TfxBytecodeOp::Merge1_3 => {
                    let [t1, t0] = stack_pop!(2);
                    stack_push!(Vec4::new(t1.x, t0.x, t0.y, t0.z));
                }
                TfxBytecodeOp::Merge2_2 => {
                    let [t1, t0] = stack_pop!(2);
                    stack_push!(Vec4::new(t1.x, t1.y, t0.x, t0.y));
                }
                TfxBytecodeOp::Unk0e => {
                    let v = stack_pop!(2);
                    stack_push!(fast_impls::byteop_0e(v))
                }
                TfxBytecodeOp::Unk0f => {
                    let [t1, t0] = stack_pop!(2);

                    stack_push!(
                        (t0.xxxx() * t1 + t0.yyyy()) * (t1 * t1) + (t0.zzzz() * t1 + t0.wwww())
                    );
                }
                TfxBytecodeOp::Lerp => {
                    let [b, a, v] = stack_pop!(3);
                    stack_push!(a + v * (b - a));
                }
                TfxBytecodeOp::Frac => {
                    let v = stack_top!();
                    *v = v.fract();
                }

                TfxBytecodeOp::PushExternInputFloat { extern_, offset } => {
                    let v = self.get_extern_float(renderer, *extern_, *offset as usize)?;
                    stack_push!(Vec4::splat(v));
                }
                TfxBytecodeOp::PushExternInputVec4 { extern_, offset } => {
                    let v = self.get_extern_vec4(renderer, *extern_, *offset as usize)?;
                    stack_push!(v);
                }
                TfxBytecodeOp::PushExternInputMat4 { extern_, offset } => {
                    let v = self.get_extern_mat4(renderer, *extern_, *offset as usize)?;
                    stack_push!(v.x_axis);
                    stack_push!(v.y_axis);
                    stack_push!(v.z_axis);
                    stack_push!(v.w_axis);
                }
                TfxBytecodeOp::PushExternInputU64 { extern_, offset } => {
                    let handle =
                        self.get_extern_u64(renderer, render_data, *extern_, *offset as usize)?;
                    let v: Vec4 = bytemuck::cast([handle, 0]);
                    stack_push!(v);
                }
                TfxBytecodeOp::PushExternInputU64Unknown { .. } => {
                    let v: Vec4 = bytemuck::cast([u64::MAX, 0]);
                    stack_push!(v);
                }
                TfxBytecodeOp::PushExternInputU32 { .. } => {
                    let v: Vec4 = bytemuck::cast([u32::MAX, 0, 0, 0]);
                    stack_push!(v);
                }
                TfxBytecodeOp::SetShaderSampler { .. } => {
                    // Just pop for now to prevent the stack from overflowing
                    let [_] = stack_pop!(1);
                }
                TfxBytecodeOp::SetShaderResource { stage, slot, .. } => {
                    let [v] = stack_pop!(1);
                    let [handle, _] = bytemuck::cast(v);
                    self.set_shader_resource(renderer, *stage, *slot as _, handle)
                }

                TfxBytecodeOp::Triangle => {
                    let v = stack_top!();
                    *v = tfx_converted::bytecode_op_triangle(*v);
                }
                TfxBytecodeOp::Jitter => {
                    let v = stack_top!();
                    *v = tfx_converted::bytecode_op_jitter(*v);
                }
                TfxBytecodeOp::Wander => {
                    let v = stack_top!();
                    *v = tfx_converted::bytecode_op_wander(*v);
                }
                TfxBytecodeOp::Rand => {
                    let v = stack_top!();
                    *v = tfx_converted::bytecode_op_rand(*v);
                }
                TfxBytecodeOp::RandSmooth => {
                    let v = stack_top!();
                    *v = tfx_converted::bytecode_op_rand_smooth(*v);
                }
                TfxBytecodeOp::TransformVec4 => {
                    let [x_axis, y_axis, z_axis, w_axis, value] = stack_pop!(5);
                    let mat = Mat4 {
                        x_axis,
                        y_axis,
                        z_axis,
                        w_axis,
                    };

                    stack_push!(mat.mul_vec4(value));
                }

                TfxBytecodeOp::Unk4c { .. }
                | TfxBytecodeOp::Unk4d { .. }
                | TfxBytecodeOp::Unk4e { .. }
                | TfxBytecodeOp::Unk4f { .. }
                | TfxBytecodeOp::Unk50 { .. }
                | TfxBytecodeOp::Unk52 { .. }
                | TfxBytecodeOp::Unk53 { .. }
                | TfxBytecodeOp::Unk54 { .. } => {
                    stack_push!(Vec4::ONE);
                }
                TfxBytecodeOp::UnkLoadConstant { constant_index } => {
                    anyhow::ensure!((*constant_index as usize) < constants.len());
                    *stack_top!() = constants[*constant_index as usize];
                }
                TfxBytecodeOp::PushConstVec4 { constant_index } => {
                    anyhow::ensure!((*constant_index as usize) < constants.len());
                    stack_push!(constants[*constant_index as usize])
                }
                TfxBytecodeOp::LerpConstant { constant_start } => {
                    anyhow::ensure!((*constant_start as usize + 1) < constants.len());
                    let a = constants[*constant_start as usize];
                    let b = constants[*constant_start as usize + 1];

                    let v = stack_top!();
                    *v = a + *v * (b - a);
                }
                TfxBytecodeOp::Unk37 { constant_start } => {
                    anyhow::ensure!((*constant_start as usize + 4) < constants.len());
                    let v = stack_top!();
                    *v = unsafe {
                        use std::arch::x86_64::*;
                        let t0: __m128 = (*v).into();
                        let v264 = _mm_cmple_ps(constants[*constant_start as usize + 4].into(), t0);
                        let v265 = _mm_and_ps(
                            _mm_add_ps(
                                _mm_mul_ps(
                                    _mm_add_ps(
                                        _mm_mul_ps(t0, constants[*constant_start as usize].into()),
                                        constants[*constant_start as usize + 1].into(),
                                    ),
                                    _mm_mul_ps(t0, t0),
                                ),
                                _mm_add_ps(
                                    _mm_mul_ps(constants[*constant_start as usize + 2].into(), t0),
                                    constants[*constant_start as usize + 3].into(),
                                ),
                            ),
                            _mm_xor_ps(
                                v264,
                                _mm_castsi128_ps(_mm_srli_si128(_mm_castps_si128(v264), 4)),
                            ),
                        );
                        let v266 = _mm_xor_ps(_mm_shuffle_ps(v265, v265, 78), v265);
                        _mm_xor_ps(_mm_shuffle_ps(v266, v266, 27), v266).into()
                    };
                }
                TfxBytecodeOp::PermuteExtendX => {
                    let v = stack_top!();
                    *v = v.xxxx();
                }
                TfxBytecodeOp::Permute { fields } => {
                    let s0 = (fields >> 6) & 0b11;
                    let s1 = (fields >> 4) & 0b11;
                    let s2 = (fields >> 2) & 0b11;
                    let s3 = fields & 0b11;

                    let v = stack_top!();
                    let v2 = v.to_array();

                    *v = Vec4::new(
                        v2[s0 as usize],
                        v2[s1 as usize],
                        v2[s2 as usize],
                        v2[s3 as usize],
                    );
                }
                TfxBytecodeOp::Saturate => {
                    let v = stack_top!();
                    *v = v.clamp(Vec4::ZERO, Vec4::ONE);
                }
                TfxBytecodeOp::Min => {
                    let [t1, t0] = stack_pop!(2);
                    stack_push!(t1.min(t0))
                }
                TfxBytecodeOp::Max => {
                    let [t1, t0] = stack_pop!(2);
                    stack_push!(t1.max(t0))
                }
                TfxBytecodeOp::PushFromOutput { element } => {
                    anyhow::ensure!(
                        (*element as usize) < buffer_map.len(),
                        "Push from output element is out of range"
                    );

                    stack_push!(buffer_map[*element as usize]);
                }
                TfxBytecodeOp::PopOutput { element } => {
                    anyhow::ensure!(
                        (*element as usize) < buffer_map.len(),
                        "Pop output element is out of range"
                    );

                    buffer_map[*element as usize] = stack_pop!(1)[0];
                }
                TfxBytecodeOp::PopOutputMat4 { element } => {
                    anyhow::ensure!(
                        (*element as usize + 3) < buffer_map.len(),
                        "Pop output mat4 element is out of range"
                    );

                    let [x_axis, y_axis, z_axis, w_axis] = stack_pop!(4);

                    let start = *element as usize;
                    buffer_map[start..start + 4].copy_from_slice(&[x_axis, y_axis, z_axis, w_axis]);
                }
                TfxBytecodeOp::PushTemp { slot } => {
                    let slotu = *slot as usize;
                    anyhow::ensure!(slotu < temp.len(), "Temp slot is out of range");
                    stack_push!(temp[slotu]);
                }
                TfxBytecodeOp::PopTemp { slot } => {
                    let slotu = *slot as usize;
                    anyhow::ensure!(slotu < temp.len(), "Temp slot is out of range");
                    let [v] = stack_pop!(1);
                    temp[slotu] = v;
                }
                #[cfg(not(feature = "tfx_strict_interpreter"))]
                _ => {}
                #[cfg(feature = "tfx_strict_interpreter")]
                u => {
                    anyhow::bail!("Unimplemented TFX bytecode op '{u:?}' at IP {ip}")
                }
            }
        }

        Ok(())
    }

    pub fn get_extern_float(
        &self,
        renderer: &Renderer,
        extern_: TfxExtern,
        offset: usize,
    ) -> anyhow::Result<f32> {
        match extern_ {
            TfxExtern::Frame => match offset {
                0 => Ok(renderer.start_time.elapsed().as_secs_f32()),

                // TODO(cohae): wrooong
                1 => Ok(renderer.start_time.elapsed().as_secs_f32()),
                4 => Ok(renderer.start_time.elapsed().as_secs_f32()),

                // Light mul
                7 => Ok(*renderer.light_mul.read()),

                _ => {
                    anyhow::bail!(
                        "get_extern_float: Unsupported extern {extern_:?}+{offset} (0x{:0X})",
                        offset * 4
                    )
                }
            },
            TfxExtern::DeferredLight => match offset {
                4 => Ok(1.0),
                8 => Ok(1.0),
                68 => Ok(1.0),
                72 => Ok(1.0),
                u => {
                    anyhow::bail!(
                        "get_extern_float: Unsupported deferred light extern offset {u} (0x{:0X})",
                        u * 16
                    )
                }
            },
            u => {
                anyhow::bail!(
                    "get_extern_float: Unsupported extern {u:?}+{offset} (0x{:0X})",
                    offset * 4
                )
            }
        }
    }

    pub fn get_extern_vec4(
        &self,
        renderer: &Renderer,
        extern_: TfxExtern,
        offset: usize,
    ) -> anyhow::Result<Vec4> {
        Ok(match extern_ {
            TfxExtern::Frame => match offset {
                // 26.x is something to do with alpha clipping. We keep it disabled, as enabling it causes a fuzzy alpha clip pattern where we dont want it
                26 => Vec4::ZERO,
                27 => Vec4::ONE,
                u => {
                    anyhow::bail!(
                        "get_extern_vec4: Unsupported frame extern offset {u} (0x{:0X})",
                        u * 16
                    )
                }
            },
            TfxExtern::Deferred => match offset {
                0 => Vec4::splat(1.0),
                u => {
                    anyhow::bail!(
                        "get_extern_vec4: Unsupported deferred extern offset {u} (0x{:0X})",
                        u * 16
                    )
                }
            },
            TfxExtern::Atmosphere => match offset {
                7 => Vec4::splat(1.0),
                u => {
                    anyhow::bail!(
                        "get_extern_vec4: Unsupported atmosphere extern offset {u} (0x{:0X})",
                        u * 16
                    )
                }
            },
            TfxExtern::DeferredLight => match offset {
                4 => Vec4::splat(1.0),
                12 => renderer.light_transform.read().translation.extend(1.0),
                13 => Vec4::splat(1.0),
                14 => Vec4::splat(1.0),
                15 => Vec4::splat(1.0),
                16 => Vec4::splat(1.0),
                u => {
                    anyhow::bail!(
                        "get_extern_vec4: Unsupported deferred light extern offset {u} (0x{:0X})",
                        u * 16
                    )
                }
            },
            u => {
                anyhow::bail!(
                    "get_extern_vec4: Unsupported extern {u:?}+{offset} (0x{:0X})",
                    offset * 16
                )
            }
        })
    }

    pub fn get_extern_mat4(
        &self,
        renderer: &Renderer,
        extern_: TfxExtern,
        offset: usize,
    ) -> anyhow::Result<Mat4> {
        Ok(match extern_ {
            TfxExtern::SimpleGeometry => match offset {
                0 => {
                    let light_transform = renderer.light_transform.read();
                    let slight_scale = *renderer.light_mat.read();

                    let viewproj = *renderer.camera_viewproj.read();

                    viewproj * (light_transform.to_mat4() * slight_scale)
                }
                u => {
                    anyhow::bail!(
                        "get_extern_mat4: Unsupported simple geometry extern offset {u} (0x{:0X})",
                        u * 16
                    )
                }
            },
            TfxExtern::DeferredLight => match offset {
                // TODO(cohae): Used for transforming projective textures
                4 => Mat4::from_scale(Vec3::splat(0.15)),
                8 => renderer.light_transform.read().to_mat4(),
                u => {
                    anyhow::bail!(
                        "get_extern_mat4: Unsupported deferred light extern offset {u} (0x{:0X})",
                        u * 16
                    )
                }
            },
            TfxExtern::View => match offset {
                0 => Mat4::IDENTITY,
                16 => Mat4::IDENTITY,
                40 => *renderer.camera_svp_inv.read(),
                u => {
                    anyhow::bail!(
                        "get_extern_mat4: Unsupported view extern offset {u} (0x{:0X})",
                        u * 16
                    )
                }
            },
            u => {
                anyhow::bail!(
                    "get_extern_mat4: Unsupported extern {u:?}+{offset} (0x{:0X})",
                    offset * 16
                )
            }
        })
    }

    pub fn get_extern_u64(
        &self,
        renderer: &Renderer,
        render_data: &RenderData,
        extern_: TfxExtern,
        offset: usize,
    ) -> anyhow::Result<u64> {
        unsafe {
            Ok(match extern_ {
                TfxExtern::Frame => match offset {
                    24 => {
                        if let Some(ir_lookup) = &render_data.iridescence_lookup {
                            transmute(ir_lookup.view.clone())
                        } else {
                            transmute(render_data.fallback_texture.view.clone())
                        }
                    }

                    u => {
                        anyhow::bail!(
                            "get_extern_u64: Unsupported frame extern offset {u} (0x{:0X})",
                            u * 8
                        )
                    }
                },
                TfxExtern::Deferred => match offset {
                    7 => transmute(renderer.gbuffer.depth.texture_view.clone()),
                    9 => transmute(renderer.gbuffer.rt0.view.clone()),
                    10 => transmute(renderer.gbuffer.rt1.view.clone()),
                    11 => transmute(renderer.gbuffer.rt2.view.clone()),
                    12 => transmute(renderer.gbuffer.light_diffuse.view.clone()),
                    13 => transmute(renderer.gbuffer.light_specular.view.clone()),
                    14 => transmute(render_data.black.view.clone()), // light_ibl_specular

                    u => {
                        anyhow::bail!(
                            "get_extern_u64: Unsupported deferred extern offset {u} (0x{:0X})",
                            u * 8
                        )
                    }
                },
                TfxExtern::Decal => match offset {
                    1 => transmute(renderer.gbuffer.rt1_clone.view.clone()),
                    u => {
                        anyhow::bail!(
                            "get_extern_u64: Unsupported decal extern offset {u} (0x{:0X})",
                            u * 8
                        )
                    }
                },
                TfxExtern::Atmosphere => match offset {
                    11 => transmute(render_data.debug_textures[1].view.clone()),
                    28 => transmute(render_data.blend_texture.view.clone()),
                    // 28 => transmute(render_data.debug_textures[2].view.clone()),
                    u => {
                        anyhow::bail!(
                            "get_extern_u64: Unsupported atmosphere extern offset {u} (0x{:0X})",
                            u * 8
                        )
                    }
                },
                TfxExtern::WaterDisplacement => match offset {
                    0 => transmute(render_data.debug_textures[0].view.clone()),
                    u => {
                        anyhow::bail!(
                            "get_extern_u64: Unsupported water displacement extern offset {u} (0x{:0X})",
                            u * 8
                        )
                    }
                },
                TfxExtern::ShadowMask => match offset {
                    1 => transmute(render_data.debug_textures[7].view.clone()),
                    u => {
                        anyhow::bail!(
                            "get_extern_u64: Unsupported shadow mask extern offset {u} (0x{:0X})",
                            u * 8
                        )
                    }
                },
                u => {
                    anyhow::bail!(
                        "get_extern_u64: Unsupported extern {u:?}+{offset} (0x{:0X})",
                        offset * 8
                    )
                }
            })
        }
    }

    pub fn dump(&self, constants: &[Vec4], buffer: &ConstantBufferCached<Vec4>) {
        debug!("Dumping TFX interpreter");
        debug!("- cb0 size: {} elements", buffer.data_array().len());
        if !constants.is_empty() {
            debug!("- Constant table:");
            for (i, v) in constants.iter().enumerate() {
                debug!("\t{i} = {v:?}")
            }
        }

        // debug!("- Stack:");
        // for (i, v) in self.stack_backup.iter().enumerate() {
        //     debug!("\t{i} = {v:?}")
        // }

        debug!("- Bytecode:");
        for (i, op) in self.opcodes.iter().enumerate() {
            debug!("\t{i}: {}", op.disassemble(Some(constants)));
        }
    }

    pub fn set_shader_resource(
        &self,
        renderer: &Renderer,
        stage: TfxShaderStage,
        slot: u32,
        handle: u64,
    ) {
        unsafe {
            match stage {
                TfxShaderStage::Pixel => renderer
                    .dcs
                    .context()
                    .PSSetShaderResources(slot, Some(&[Some(std::mem::transmute(handle))])),
                TfxShaderStage::Vertex => renderer
                    .dcs
                    .context()
                    .PSSetShaderResources(slot, Some(&[Some(std::mem::transmute(handle))])),
                TfxShaderStage::Geometry => renderer
                    .dcs
                    .context()
                    .PSSetShaderResources(slot, Some(&[Some(std::mem::transmute(handle))])),
                TfxShaderStage::Hull => renderer
                    .dcs
                    .context()
                    .PSSetShaderResources(slot, Some(&[Some(std::mem::transmute(handle))])),
                TfxShaderStage::Compute => renderer
                    .dcs
                    .context()
                    .PSSetShaderResources(slot, Some(&[Some(std::mem::transmute(handle))])),
                TfxShaderStage::Domain => renderer
                    .dcs
                    .context()
                    .PSSetShaderResources(slot, Some(&[Some(std::mem::transmute(handle))])),
            }
        }
    }
}

#[allow(non_snake_case)]
mod fast_impls {
    use glam::Vec4;
    use std::arch::x86_64::*;

    pub fn byteop_0e([t1, t0]: [Vec4; 2]) -> Vec4 {
        unsafe {
            let xmmword_7FF7B2E5E4F0 = _mm_castsi128_ps(_mm_setr_epi32(
                u32::MAX as _,
                u32::MAX as _,
                u32::MAX as _,
                0,
            ));
            let xmmword_7FF7B2E5E5C0 = _mm_castsi128_ps(_mm_setr_epi32(0, 0, 0, u32::MAX as _));
            let xmmword_7FF7B2E5E4E0 = _mm_set1_ps(f32::NAN);

            _mm_add_ps(
                _mm_or_ps(
                    _mm_and_ps(
                        _mm_and_ps(
                            _mm_shuffle_ps(t0.into(), t0.into(), 0),
                            xmmword_7FF7B2E5E5C0,
                        ),
                        xmmword_7FF7B2E5E4E0,
                    ),
                    _mm_andnot_ps(xmmword_7FF7B2E5E4E0, _mm_set1_ps(1.0)),
                ),
                _mm_and_ps(t1.into(), xmmword_7FF7B2E5E4F0),
            )
            .into()
        }
    }
}

// Methods adapted from HLSL TFX sources
mod tfx_converted {
    use glam::{Vec4, Vec4Swizzles};

    fn lerp(start: f32, end: f32, t: f32) -> f32 {
        start + (end - start) * t
    }

    fn _trig_helper_vector_pseudo_sin_rotations_clamped(a: Vec4) -> Vec4 {
        a * (a.abs() * -16.0 + 8.0)
    }

    fn _trig_helper_vector_pseudo_sin_rotations(a: Vec4) -> Vec4 {
        let w = a - a.round(); // wrap to [-0.5, 0.5] range
        _trig_helper_vector_pseudo_sin_rotations_clamped(w)
    }

    pub fn bytecode_op_triangle(x: Vec4) -> Vec4 {
        let wrapped = x - x.round(); // wrap to [-0.5, 0.5] range
        let abs_wrap = wrapped.abs(); // abs turns into triangle wave between [0, 0.5]

        abs_wrap * 2.0 // scale to [0, 1] range
    }

    pub fn bytecode_op_jitter(x: Vec4) -> Vec4 {
        let rotations =
            x.xxxx() * Vec4::new(4.67, 2.99, 1.08, 1.35) + Vec4::new(0.52, 0.37, 0.16, 0.79);

        // optimized scaled-sum-of-sines
        let a = rotations - rotations.round(); // wrap to [-0.5, 0.5] range
        let ma = a.abs() * -16.0 + 8.0;
        let sa = a * 0.25;
        let v = sa.dot(ma) + 0.5;

        // hermite smooth interpolation (3*v^2 - 2*v^3)
        let v2 = v * v;
        let jitter_result = (-2.0 * v + 3.0) * v2;

        Vec4::splat(jitter_result)
    }

    pub fn bytecode_op_wander(x: Vec4) -> Vec4 {
        let rot0 = x.xxxx() * Vec4::new(4.08, 1.02, 3.0 / 5.37, 3.0 / 9.67)
            + Vec4::new(0.92, 0.33, 0.26, 0.54);
        let rot1 = x.xxxx() * Vec4::new(1.83, 3.09, 0.39, 0.87) + Vec4::new(0.12, 0.37, 0.16, 0.79);
        let sines0 = _trig_helper_vector_pseudo_sin_rotations(rot0);
        let sines1 =
            _trig_helper_vector_pseudo_sin_rotations(rot1) * Vec4::new(0.02, 0.02, 0.28, 0.28);
        let wander_result = 0.5 + sines0.dot(sines1);

        Vec4::splat(wander_result)
    }

    pub fn bytecode_op_rand(x: Vec4) -> Vec4 {
        // these magic numbers are 1/(prime/1000000)
        let v0 = x.x.floor();
        let mut val0 = Vec4::splat(v0).dot(Vec4::new(
            1.0 / 1.043501,
            1.0 / 0.794471,
            1.0 / 0.113777,
            1.0 / 0.015101,
        ));
        val0 = val0.fract();

        //			val0=	bbs(val0);		// Blum-Blum-Shub randomimzer
        val0 = val0 * val0 * 251.0;
        val0 = val0.fract();

        Vec4::splat(val0)
    }

    pub fn bytecode_op_rand_smooth(x: Vec4) -> Vec4 {
        let v = x.x;
        let v0 = v.round();
        let v1 = v0 + 1.0;
        let f = v - v0;
        let f2 = f * f;

        // hermite smooth interpolation (3*f^2 - 2*f^3)
        let smooth_f = (-2.0 * f + 3.0) * f2;

        // these magic numbers are 1/(prime/1000000)
        let mut val0 = Vec4::splat(v0).dot(Vec4::new(
            1.0 / 1.043501,
            1.0 / 0.794471,
            1.0 / 0.113777,
            1.0 / 0.015101,
        ));
        let mut val1 = Vec4::splat(v1).dot(Vec4::new(
            1.0 / 1.043501,
            1.0 / 0.794471,
            1.0 / 0.113777,
            1.0 / 0.015101,
        ));

        val0 = val0.fract();
        val1 = val1.fract();

        //			val0=	bbs(val0);		// Blum-Blum-Shub randomimzer
        val0 = val0 * val0 * 251.0;
        val0 = val0.fract();

        //			val10=	bbs(val1);		// Blum-Blum-Shub randomimzer
        val1 = val1 * val1 * 251.0;
        val1 = val1.fract();

        let rand_smooth_result = lerp(val0, val1, smooth_f);

        Vec4::splat(rand_smooth_result)
    }

    pub fn _trig_helper_vector_sin_rotations_estimate_clamped(a: Vec4) -> Vec4 {
        let y = a * (-16.0 * a.abs() + 8.0);
        y * (0.225 * y.abs() + 0.775)
    }

    pub fn _trig_helper_vector_sin_rotations_estimate(a: Vec4) -> Vec4 {
        let w = a - a.round(); // wrap to [-0.5, 0.5] range
        _trig_helper_vector_sin_rotations_estimate_clamped(w)
    }

    pub fn _trig_helper_vector_cos_rotations_estimate(a: Vec4) -> Vec4 {
        _trig_helper_vector_sin_rotations_estimate(a + 0.25)
    }

    pub fn _trig_helper_vector_sin_cos_rotations_estimate(a: Vec4) -> Vec4 {
        _trig_helper_vector_sin_rotations_estimate(a + Vec4::new(0.0, 0.25, 0.0, 0.25))
    }
}
