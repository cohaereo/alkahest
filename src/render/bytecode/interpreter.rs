use glam::Vec4;
use windows::Win32::Graphics::Direct3D11::D3D11_MAP_WRITE_NO_OVERWRITE;

use crate::render::{renderer::Renderer, ConstantBuffer};

use super::{externs::TfxExtern, opcodes::TfxBytecodeOp};

pub struct TfxBytecodeInterpreter {
    opcodes: Vec<TfxBytecodeOp>,
    stack: Vec<Vec4>,
    operand_stack: Vec<Vec4>,
}

impl TfxBytecodeInterpreter {
    pub fn new(opcodes: Vec<TfxBytecodeOp>) -> Self {
        Self {
            opcodes,
            stack: Vec::with_capacity(8),
            operand_stack: Vec::with_capacity(8),
        }
    }

    pub fn evaluate(
        &mut self,
        renderer: &Renderer,
        buffer: &ConstantBuffer<Vec4>,
        constants: &[Vec4],
    ) -> anyhow::Result<()> {
        self.stack.clear();

        let Ok(buffer_map) = buffer.map(D3D11_MAP_WRITE_NO_OVERWRITE) else {
            error!("Failed to map cb0 for TFX interpreter");
            return Ok(());
        };

        for (ip, op) in self.opcodes.iter().enumerate() {
            match op {
                TfxBytecodeOp::LoadExtern { extern_, element } => {
                    let v = self.get_extern(renderer, *extern_, *element)?;
                    self.stack.push(v);
                }
                TfxBytecodeOp::UnkLoadConstant { constant_index } => {
                    anyhow::ensure!((*constant_index as usize) < constants.len());
                    self.stack.push(constants[*constant_index as usize]);
                }
                TfxBytecodeOp::UnkLoadConstant2 { constant_index } => {
                    anyhow::ensure!((*constant_index as usize) < constants.len());
                    self.operand_stack.push(constants[*constant_index as usize]);
                }
                // TODO(cohae): Seems to be an arithmetic op
                TfxBytecodeOp::Unk03 => {
                    if let (Some(v1), Some(v2)) = (self.stack.pop(), self.operand_stack.pop()) {
                        self.stack.push(v1 * v2);
                    }
                }
                TfxBytecodeOp::StoreToBuffer { element } => {
                    if let Some(value) = self.stack.pop() {
                        unsafe { buffer_map.ptr.offset(*element as isize).write(value) }
                    }
                }
                // #[cfg(feature = "tfx_experimental_opcodes")]
                // TfxBytecodeOp::
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
                    renderer.delta_time,
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

        debug!("- Stack:");
        for (i, v) in self.stack.iter().enumerate() {
            debug!("\t{i} = {v:?}")
        }

        debug!("- Bytecode:");
        for (i, op) in self.opcodes.iter().enumerate() {
            debug!("\t{i}: {}", op.disassemble());
        }
    }
}
