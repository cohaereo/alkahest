use std::mem::transmute;

use glam::{Mat4, Vec4, Vec4Swizzles};
use tinyvec::ArrayVec;
use windows::Win32::Graphics::Direct3D11::D3D11_MAP_WRITE_NO_OVERWRITE;

use crate::render::{renderer::Renderer, ConstantBuffer, RenderData};

use super::{
    externs::{TfxExtern, TfxShaderStage},
    opcodes::TfxBytecodeOp,
};

pub struct TfxBytecodeInterpreter {
    opcodes: Vec<TfxBytecodeOp>,
}

impl TfxBytecodeInterpreter {
    pub fn new(opcodes: Vec<TfxBytecodeOp>) -> Self {
        Self { opcodes }
    }

    pub fn evaluate(
        &self,
        renderer: &Renderer,
        render_data: &RenderData,
        buffer: &ConstantBuffer<Vec4>,
        constants: &[Vec4],
    ) -> anyhow::Result<()> {
        let mut stack: ArrayVec<[Vec4; 64]> = Default::default();
        let mut temp = [Vec4::ZERO; 16];

        let Ok(buffer_map) = buffer.map(D3D11_MAP_WRITE_NO_OVERWRITE) else {
            error!("Failed to map cb0 for TFX interpreter");
            return Ok(());
        };

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

        for (_ip, op) in self.opcodes.iter().enumerate() {
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
                TfxBytecodeOp::IsZero => {
                    // Cleaned up SIMD: _mm_and_ps(_mm_cmpeq_ps(a, _mm_setzero_ps()), _mm_set1_ps(1.0));
                    // Decompiled and simplified: value == 0.0 ? 1.0 : 0.0 (for each element in the vector)
                    let v = stack_top!();
                    let c = v.cmpeq(Vec4::ZERO);
                    *v = Vec4::new(
                        c.test(0) as u32 as f32,
                        c.test(1) as u32 as f32,
                        c.test(2) as u32 as f32,
                        c.test(3) as u32 as f32,
                    );
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
                    *v = -*v;
                }
                TfxBytecodeOp::Cosine => {
                    let v = stack_top!();
                    *v = Vec4::new(v.x.cos(), v.y.cos(), v.z.cos(), v.w.cos());
                }
                TfxBytecodeOp::Unk0b => {
                    let [t1, t0] = stack_pop!(2);
                    let v = unsafe {
                        use std::arch::x86_64::*;
                        let v26 = _mm_mul_ps(t1.into(), t0.into());
                        // TODO(cohae): Figure out swizzles after i regain my sanity
                        let v27 = _mm_add_ps(_mm_shuffle_ps(v26, v26, 0b01001110), v26);
                        _mm_add_ps(_mm_shuffle_ps(v27, v27, 0b10010011), v27)
                    };
                    stack_push!(Vec4::from(v));
                }
                // TODO(cohae): Is the parameter order for merges correct?
                TfxBytecodeOp::Merge1_3 => {
                    let [t1, t0] = stack_pop!(2);
                    stack_push!(Vec4::new(t1.x, t0.x, t0.y, t0.z));
                }
                TfxBytecodeOp::Merge2_2 => {
                    let [t1, t0] = stack_pop!(2);
                    stack_push!(Vec4::new(t1.x, t1.y, t0.x, t0.y));
                }
                TfxBytecodeOp::Unk0f => {
                    let [t1, t0] = stack_pop!(2);

                    let v = unsafe {
                        use std::arch::x86_64::*;
                        let a = t1.into();
                        let b = t0.into();
                        _mm_add_ps(
                            _mm_mul_ps(
                                _mm_add_ps(
                                    _mm_mul_ps(_mm_shuffle_ps(b, b, 0), a),
                                    _mm_shuffle_ps(b, b, 85),
                                ),
                                _mm_mul_ps(a, a),
                            ),
                            _mm_add_ps(
                                _mm_mul_ps(_mm_shuffle_ps(b, b, 170), a),
                                _mm_shuffle_ps(b, b, 255),
                            ),
                        )
                    };

                    stack_push!(Vec4::from(v));
                }
                // TODO(cohae): Check the SIMD output again, seems like frac but not really
                TfxBytecodeOp::Unk1a => {
                    let v = stack_top!();
                    // *v = fast_impls::byteop_1a([*v]);
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
                TfxBytecodeOp::PushExternInputMat4 { .. } => {
                    let mat = Mat4::IDENTITY;
                    stack_push!(mat.x_axis);
                    stack_push!(mat.y_axis);
                    stack_push!(mat.z_axis);
                    stack_push!(mat.w_axis);
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

                TfxBytecodeOp::Unk27 => {
                    let v = stack_top!();
                    *v = tfx_converted::bytecode_op_triangle(*v);
                }
                TfxBytecodeOp::Unk28 => {
                    let v = stack_top!();
                    *v = tfx_converted::bytecode_op_jitter(*v);
                }
                TfxBytecodeOp::Unk29 => {
                    let v = stack_top!();
                    *v = tfx_converted::bytecode_op_wander(*v);
                }
                TfxBytecodeOp::Unk2a => {
                    let v = stack_top!();
                    *v = tfx_converted::bytecode_op_rand(*v);
                }
                TfxBytecodeOp::Unk2b => {
                    let v = stack_top!();
                    *v = tfx_converted::bytecode_op_rand_smooth(*v);
                }

                TfxBytecodeOp::Unk4c { .. }
                | TfxBytecodeOp::Unk4d { .. }
                | TfxBytecodeOp::Unk4e { .. }
                | TfxBytecodeOp::Unk4f { .. } => {
                    stack_push!(Vec4::ONE);
                    // stack_push!(Vec4::ZERO);
                }
                TfxBytecodeOp::UnkLoadConstant { constant_index } => {
                    anyhow::ensure!((*constant_index as usize) < constants.len());
                    *stack_top!() = constants[*constant_index as usize];
                }
                TfxBytecodeOp::PushConstVec4 { constant_index } => {
                    anyhow::ensure!((*constant_index as usize) < constants.len());
                    stack_push!(constants[*constant_index as usize])
                }
                TfxBytecodeOp::Unk35 { constant_start } => {
                    anyhow::ensure!((*constant_start as usize + 1) < constants.len());
                    let v1 = constants[*constant_start as usize];
                    let v2 = constants[*constant_start as usize + 1];

                    let v = stack_top!();
                    *v = ((v2 - v1) * *v) + v1;
                }
                // // TODO(cohae): Very wrong, but does seem to push something onto the stack
                TfxBytecodeOp::PermuteAllX => {
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
                TfxBytecodeOp::PopOutput { element } => unsafe {
                    buffer_map
                        .ptr
                        .offset(*element as isize)
                        .write(stack_pop!(1)[0])
                },
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
                    anyhow::bail!("Unimplemented TFX bytecode op '{u:?}' at IP {_ip}")
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

                _ => {
                    anyhow::bail!(
                        "get_extern_float: Unsupported extern {extern_:?}+{offset} (0x{:0X})",
                        offset * 4
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
        _renderer: &Renderer,
        extern_: TfxExtern,
        offset: usize,
    ) -> anyhow::Result<Vec4> {
        Ok(match extern_ {
            TfxExtern::Frame => match offset {
                // 26.x is something to do with alpha clipping. We keep it disabled, as enabling it causes a fuzzy alpha clip pattern where we dont want it
                26 => Vec4::ZERO,
                u => {
                    anyhow::bail!(
                        "get_extern_vec4: Unsupported frame extern offset {u} (0x{:0X})",
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

    pub fn get_extern_u64(
        &self,
        _renderer: &Renderer,
        render_data: &RenderData,
        extern_: TfxExtern,
        offset: usize,
    ) -> anyhow::Result<u64> {
        unsafe {
            Ok(match extern_ {
                TfxExtern::Deferred => match offset {
                    7 => transmute(render_data.debug_textures[7].view.clone()),
                    u => {
                        anyhow::bail!(
                            "get_extern_u64: Unsupported deferred extern offset {u} (0x{:0X})",
                            u * 16
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
                            u * 16
                        )
                    }
                },
                TfxExtern::WaterDisplacement => match offset {
                    0 => transmute(render_data.debug_textures[0].view.clone()),
                    u => {
                        anyhow::bail!(
                            "get_extern_u64: Unsupported water displacement extern offset {u} (0x{:0X})",
                            u * 16
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

    pub fn dump(&self, constants: &[Vec4], buffer: &ConstantBuffer<Vec4>) {
        debug!("Dumping TFX interpreter");
        debug!("- cb0 size: {} elements", buffer.elements());
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
            debug!("\t{i}: {}", op.disassemble());
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

// mod fast_impls {
//     use glam::Vec4;
//     use std::arch::x86_64::*;

//     pub fn byteop_1a([t0]: [Vec4; 1]) -> Vec4 {
//         unsafe {
//             let v49 = _mm_cmpgt_epi32(
//                 _mm_castps_si128(_mm_set1_ps(8388608.0)),
//                 _mm_castps_si128(_mm_and_ps(_mm_set1_ps(f32::NAN), t0.into())),
//             );
//             let v50 = _mm_cvtepi32_ps(_mm_cvttps_epi32(t0.into()));

//             _mm_sub_ps(
//                 t0.into(),
//                 _mm_or_ps(
//                     _mm_and_ps(
//                         _mm_sub_ps(
//                             v50,
//                             _mm_and_ps(_mm_cmplt_ps(t0.into(), v50), _mm_set1_ps(1.0)),
//                         ),
//                         _mm_castsi128_ps(v49),
//                     ),
//                     _mm_castsi128_ps(_mm_andnot_si128(v49, _mm_castps_si128(t0.into()))),
//                 ),
//             )
//             .into()
//         }
//     }
// }

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
}
