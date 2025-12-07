// use glam::Vec4;

// use crate::{gpu::command_list::CommandList, tfx::expression_vm::interpreter::InterpreterState};

// use super::opcodes::Opcode;

// fn test_setup() -> (d3d11::Device, d3d11::DeviceContext) {
//     let device = d3d11::Device::create(None, false).expect("Failed to create device");
//     let context = device.get_immediate_context();
//     (device, context)
// }

// #[test]
// fn simple() {
//     let (_device, context) = test_setup();
//     #[rustfmt::skip]
//     let bytecode: &[u8] = &[
//         Opcode::PushConstVec4 as u8, 0, // push constant 0
//         Opcode::PushConstVec4 as u8, 1, // push constant 1
//         Opcode::Add as u8, // add
//         Opcode::PopOutput as u8, 0, // pop to first element
//     ];

//     let constants = &[Vec4::new(1.0, 2.0, 3.0, 4.0), Vec4::new(5.0, 6.0, 7.0, 8.0)];
//     let mut out = [Vec4::ZERO; 1];
//     InterpreterState::new(bytecode)
//         .evaluate(&context, constants, &[], &mut out)
//         .expect("Failed to evaluate");

//     assert!(out[0].abs_diff_eq(Vec4::new(6.0, 8.0, 10.0, 12.0), 0.0001));
// }
