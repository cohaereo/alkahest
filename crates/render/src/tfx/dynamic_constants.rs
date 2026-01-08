use std::sync::Arc;

use alkahest_data::tfx::{SDynamicConstants, ShaderStage};
use anyhow::Context;
use glam::Vec4;
use itertools::Itertools;
use tiger_pkg::package_manager;

use super::expression_vm::{
    self,
    interpreter::{InterpreterState, TempObjectChannels},
};
use crate::{
    Gpu, Renderer,
    asset::{
        Handle,
        texture::{Texture, load_sampler},
    },
    gpu::{
        cbuffer::ConstantBuffer,
        command_list::{CommandList, ContextExt},
    },
    tfx::expression_vm::opcodes::{Opcode, OpcodeIterator},
};

/// Holds all dynamically bound resources for a shader
pub struct DynamicConstants {
    pub textures: Vec<(u32, Option<Handle<Texture>>)>,
    pub samplers: Vec<Option<d3d11::SamplerState>>,
    pub cbuffer_slot: u32,
    pub cbuffer: Option<ConstantBuffer<Vec4>>,
    pub bytecode: Vec<u8>,
    pub bytecode_constants: Vec<Vec4>,

    pub initial_constants: Vec<Vec4>,

    /// Indicates if the expression bytecode writes to the constant buffer. If this is false, then the cbuffer is not mapped for writing.
    writes_cbuffer: bool,
}

impl DynamicConstants {
    pub fn load(gpu: &Arc<Gpu>, constants: &SDynamicConstants) -> anyhow::Result<Self> {
        let (initial_constants, cbuffer) = if constants.constant_buffer.is_some() {
            let entry = package_manager()
                .get_entry(constants.constant_buffer)
                .context("Failed to get cbuffer tag entry")?;

            let data = package_manager().read_tag(entry.reference)?;
            let vec4s = bytemuck::cast_slice(&data);
            let cb = ConstantBuffer::create_array(gpu, vec4s.len(), Some(vec4s))?;
            (vec4s.to_vec(), Some(cb))
        } else {
            let vec4s = &constants.unk30;
            if vec4s.is_empty() {
                (vec![], None)
            } else {
                let cb = ConstantBuffer::create_array(gpu, vec4s.len(), Some(vec4s))?;
                (vec4s.to_vec(), Some(cb))
            }
        };

        let writes_cbuffer = OpcodeIterator::new(&constants.bytecode).any(|op| {
            matches!(
                op,
                (Opcode::PopOutput, _) | (Opcode::PopOutputMat4, _) | (Opcode::PushFromOutput, _)
            )
        });

        Ok(Self {
            textures: constants
                .textures
                .iter()
                .map(|tex| {
                    (
                        tex.slot,
                        Renderer::instance()
                            .asset_manager
                            .try_load(tex.texture.hash32()),
                    )
                })
                .collect(),
            samplers: constants
                .samplers
                .iter()
                .map(|sampler| {
                    if sampler.sampler.is_none() {
                        error!("Sampler is None");
                        Ok(None)
                    } else {
                        let sampler = load_sampler(gpu, sampler.sampler)?;
                        Ok(Some(sampler))
                    }
                })
                .collect::<anyhow::Result<_>>()?,
            cbuffer_slot: constants.constant_buffer_slot as u32,
            cbuffer,
            bytecode: constants.bytecode.clone(),
            bytecode_constants: constants.bytecode_constants.clone(),

            initial_constants,

            writes_cbuffer,
        })
    }

    #[profiling::function]
    fn prepare_constants(
        &self,
        cmd: &mut CommandList,
        channels: Option<&TempObjectChannels>,
    ) -> anyhow::Result<()> {
        if self.writes_cbuffer {
            if let Some(ref cbuffer) = self.cbuffer {
                let map = unsafe {
                    cmd.map_unchecked(cbuffer.buffer(), 0, d3d11::MapType::WriteDiscard, false)?
                };
                let data = unsafe {
                    std::slice::from_raw_parts_mut(map.data as *mut Vec4, cbuffer.size() / 16)
                };

                // Copy the initial constants
                data[..self.initial_constants.len()].copy_from_slice(&self.initial_constants);
                self.evaluate_expressions(cmd, Some(data), channels);
                cmd.unmap(cbuffer.buffer(), 0);
            }
        } else {
            self.evaluate_expressions(cmd, None, channels);
        }

        Ok(())
    }

    fn evaluate_expressions(
        &self,
        cmd: &mut CommandList,
        output: Option<&mut [Vec4]>,
        channels: Option<&TempObjectChannels>,
    ) {
        // profiling::scope!(
        //     "evaluate_expression_bytecode",
        //     &format!("bytes={}", self.bytecode.len())
        // );

        let mut interpreter = InterpreterState::new(&self.bytecode)
            .with_d3d11_context(cmd)
            .with_externs(&cmd.externs);
        if let Some(channels) = channels {
            interpreter = interpreter.with_object_channels(channels);
        }
        if let Err(e) = interpreter.evaluate(
            &self.bytecode_constants,
            &self.samplers,
            output.unwrap_or(&mut []),
        ) {
            error!("Failed to evaluate expression bytecode: {:?}", e);

            let bytecode_listing = match expression_vm::disassemble(&self.bytecode) {
                Ok(ops) => ops.into_iter().map(|v| format!("    {v}")).join("\n"),
                Err(e) => {
                    format!("Failed to disassemble bytecode: {e:?}")
                }
            };
            debug!("Bytecode:\n{}", bytecode_listing);

            if interpreter.ip < self.bytecode.len() {
                // Patch the bytecode to disable the expression
                unsafe {
                    self.bytecode
                        .as_ptr()
                        .add(interpreter.ip)
                        .cast_mut()
                        .write(expression_vm::opcodes::Opcode::ExtReturn as u8);
                }
            }
        }
    }

    #[profiling::function]
    pub fn bind(
        &self,
        cmd: &mut CommandList,
        stage: ShaderStage,
        channels: Option<&TempObjectChannels>,
    ) -> anyhow::Result<()> {
        if !self.bytecode.is_empty() {
            self.prepare_constants(cmd, channels)?;
        }

        if self.cbuffer_slot != u32::MAX {
            if let Some(ref cbuffer) = self.cbuffer {
                cbuffer.bind(cmd, stage, self.cbuffer_slot);
            } else {
                cmd.set_constant_buffer(stage, self.cbuffer_slot, None);
            }
        }

        for &(slot, ref tex) in self.textures.iter() {
            if let Some(tex) = tex.as_ref().and_then(|t| t.get()) {
                tex.bind(cmd, slot, stage);
            } else {
                cmd.set_shader_resource(stage, slot, None);
            }
        }

        Ok(())
    }
}
