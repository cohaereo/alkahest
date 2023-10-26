use glam::{Vec4, Vec4Swizzles};
use tinyvec::ArrayVec;
use windows::Win32::Graphics::Direct3D11::D3D11_MAP_WRITE_NO_OVERWRITE;

use crate::render::{renderer::Renderer, ConstantBuffer};

use super::{externs::TfxExtern, opcodes::TfxBytecodeOp};

pub struct TfxBytecodeInterpreter {
    opcodes: Vec<TfxBytecodeOp>,
}

impl TfxBytecodeInterpreter {
    pub fn new(opcodes: Vec<TfxBytecodeOp>) -> Self {
        Self { opcodes }
    }

    pub fn evaluate(
        &mut self,
        renderer: &Renderer,
        buffer: &ConstantBuffer<Vec4>,
        constants: &[Vec4],
    ) -> anyhow::Result<()> {
        let mut stack: ArrayVec<[Vec4; 128]> = Default::default();
        let mut temp = [Vec4::ZERO; 16];

        let Ok(buffer_map) = buffer.map(D3D11_MAP_WRITE_NO_OVERWRITE) else {
            error!("Failed to map cb0 for TFX interpreter");
            return Ok(());
        };

        // macro_rules! stack_verify {
        //     // $pops: the amount of stack elements this operation needs to read
        //     ($pops:literal) => {
        //         anyhow::ensure!(stack.len() >= $pops);
        //     };
        // }

        macro_rules! stack_pop {
            ($pops:literal) => {{
                anyhow::ensure!(!stack.is_empty() && stack.len() >= $pops);
                let v: [Vec4; $pops] = stack[stack.len() - $pops..stack.len()].try_into().unwrap();
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
                // TODO(cohae): Check the SIMD output again, seems like frac but not really
                TfxBytecodeOp::Unk1a => {
                    let v = stack_top!();
                    *v = v.fract();
                }

                // TODO(cohae): uses offset thingy, not elements
                TfxBytecodeOp::PushExternInputFloat { extern_, element } => {
                    let v = self.get_extern(renderer, *extern_, *element)?;
                    stack_push!(v);
                }
                TfxBytecodeOp::Merge1_3 => {
                    let [t1, t0] = stack_pop!(2);
                    // TODO(cohae): Is the parameter order correct?
                    stack_push!(Vec4::new(t1.x, t0.x, t0.y, t0.z));
                }
                TfxBytecodeOp::Unk3d { .. }
                | TfxBytecodeOp::Unk3f { .. }
                | TfxBytecodeOp::Unk4c { .. }
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
                // // TODO(cohae): Very wrong, but does seem to push something onto the stack
                TfxBytecodeOp::PermuteAllX => {
                    let v = stack_top!();
                    *v = v.xxxx();
                }
                TfxBytecodeOp::Permute { fields } => {
                    // TODO(cohae): reimplement using SIMD
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
                // TfxBytecodeOp::PushTemp { slot } => {
                //     let slotu = *slot as usize;
                //     anyhow::ensure!(slotu < temp.len(), "Temp slot is out of range");
                //     stack_push!(temp[slotu]);
                // }
                // TfxBytecodeOp::PopTemp { slot } => {
                //     let slotu = *slot as usize;
                //     anyhow::ensure!(slotu < temp.len(), "Temp slot is out of range");
                //     let [v] = stack_pop!(1);
                //     temp[slotu] = v;
                // }
                // #[cfg(feature = "tfx_experimental_opcodes")]
                // TfxBytecodeOp::
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

    pub fn get_extern(
        &self,
        renderer: &Renderer,
        extern_: TfxExtern,
        element: u8,
    ) -> anyhow::Result<Vec4> {
        match extern_ {
            TfxExtern::Frame => match element {
                0 => Ok(Vec4::new(
                    renderer.start_time.elapsed().as_secs_f32(),
                    renderer.start_time.elapsed().as_secs_f32(),
                    *renderer.delta_time.read(),
                    1.0,
                )),
                1 => Ok(Vec4::ONE),  // Exposure scales
                4 => Ok(Vec4::ZERO), // Stubbed
                u => anyhow::bail!("Unsupported element {u} for extern {extern_:?}"),
            },
            u => {
                anyhow::bail!("Unsupported extern {u:?}[{element}]")
            }
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
}
