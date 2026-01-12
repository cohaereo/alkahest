use glam::Vec4;

use crate::tfx::{
    externs::Externs,
    sequencer_vm::{interpreter::InterpreterState, opcodes::Opcode},
};

pub struct GlobalChannelExpression {
    pub channel_id: u32,
    pub bytecode: Vec<u8>,
    pub bytecode_constants: Vec<Vec4>,
}

impl GlobalChannelExpression {
    pub fn evaluate(&mut self, externs: &mut Externs) {
        let mut _temp_value = Vec4::ZERO;
        let (slice, _is_misc) = if let Some(pos) = externs
            .global_ids
            .iter()
            .position(|i| *i == self.channel_id)
        {
            (&mut externs.globals[pos..], false)
        } else {
            (std::slice::from_mut(&mut _temp_value), true)
        };

        let mut interpreter = InterpreterState::new(&self.bytecode);
        if let Err(e) = interpreter.evaluate(&self.bytecode_constants, slice) {
            error!(
                "Failed to evaluate global channel {:08X} bytecode: {e:?}",
                self.channel_id
            );

            let ip = interpreter.ip;
            // drop the interpreter to get the mutable ref to bytecode back
            let _ = interpreter;

            if ip < self.bytecode.len() {
                // Patch the bytecode to disable the expression
                self.bytecode[ip] = Opcode::ExtReturn as u8;
            }
        }

        // if is_misc {
        //     externs.misc_values.insert(self.channel_id, _temp_value);
        // }
    }
}
