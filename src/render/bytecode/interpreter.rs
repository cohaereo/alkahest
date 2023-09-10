use glam::Vec4;
use windows::Win32::Graphics::Direct3D11::D3D11_MAP_WRITE_NO_OVERWRITE;

use crate::render::{renderer::Renderer, ConstantBuffer};

use super::opcodes::TfxBytecodeOp;

pub struct TfxBytecodeInterpreter {
    opcodes: Vec<TfxBytecodeOp>,
    stack: Vec<Vec4>,
}

impl TfxBytecodeInterpreter {
    pub fn new(opcodes: Vec<TfxBytecodeOp>) -> Self {
        Self {
            opcodes,
            stack: Vec::with_capacity(8),
        }
    }

    pub fn evaluate(
        &mut self,
        _renderer: &Renderer,
        buffer: &ConstantBuffer<Vec4>,
        constants: &[Vec4],
    ) -> anyhow::Result<()> {
        self.stack.clear();

        let Ok(buffer_map) = buffer.map(D3D11_MAP_WRITE_NO_OVERWRITE) else {
            error!("Failed to map cb0 for TFX interpreter");
            return Ok(());
        };

        for op in &self.opcodes {
            match op {
                // TfxBytecodeOp::LoadExtern { extern_, element } => {}
                TfxBytecodeOp::LoadConstant { constant_index } => {
                    self.stack.push(constants[*constant_index as usize]);
                }
                TfxBytecodeOp::StoreToBuffer { element } => {
                    if let Some(value) = self.stack.pop() {
                        unsafe { buffer_map.ptr.offset(*element as isize).write(value) }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}
