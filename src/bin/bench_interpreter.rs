use std::time::Instant;

use alkahest_render::tfx::expression_vm::{interpreter::InterpreterState, opcodes::Opcode};
use glam::Vec4;

fn main() {
    let mut bytecode = vec![];

    {
        // push_from_output 0
        bytecode.push(Opcode::PushFromOutput as u8);
        bytecode.push(0x00);
        // push_from_output 1
        bytecode.push(Opcode::PushFromOutput as u8);
        bytecode.push(0x01);
        // cubic
        bytecode.push(Opcode::Cubic as u8);
        for _ in 0..3 {
            // jitter
            bytecode.push(Opcode::Jitter as u8);
            // triangle
            bytecode.push(Opcode::Triangle as u8);
            // wander
            bytecode.push(Opcode::Wander as u8);
        }
        // permute .xxxx
        bytecode.push(Opcode::Splat as u8);
        // pop_output 3
        bytecode.push(Opcode::PopOutput as u8);
        bytecode.push(0x02);
    }

    let start = Instant::now();
    const NUM_ITER: usize = 25_000_000;
    let mut output = [Vec4::ONE * 2.0, Vec4::new(0.25, 0.5, 0.75, 1.0), Vec4::ZERO];
    for i in 0..NUM_ITER {
        // output[1].z = i as f32 / 4.0;
        output[0].x = i as f32 / 256_000.0;
        std::hint::black_box(
            InterpreterState::new(&bytecode)
                .evaluate(&[], &[], &mut output)
                .ok(),
        );

        // println!(
        //     "unknown0xf({:?}, {:?}) = {:?}",
        //     output[0], output[1], output[2]
        // );
        // println!(
        //     "unknown0xf({:?}, {:?}) = {:?}",
        //     output[0], output[1], output[2]
        // );

        // let result = vec4(
        //     (a.x * b.x + a.y) * b.x * b.x + a.z * b.x + a.w,
        //     (a.x * b.y + a.y) * b.y * b.y + a.z * b.y + a.w,
        //     (a.x * b.z + a.y) * b.z * b.z + a.z * b.z + a.w,
        //     (a.x * b.w + a.y) * b.w * b.w + a.z * b.w + a.w,
        // );

        // println!("  ours = {:?}", result);
    }
    println!("Destiny (Alkahest):");
    let elapsed = start.elapsed();
    println!("  Elapsed time: {:?}", elapsed);
    let time_per_iter = elapsed.as_micros() as f32 / NUM_ITER as f32;
    println!("  Time per iteration: {} us", time_per_iter);
}
