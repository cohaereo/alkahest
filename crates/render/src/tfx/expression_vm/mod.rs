pub mod decompiler;
pub(crate) mod helpers;
pub mod interpreter;
pub mod opcodes;
#[cfg(test)]
pub mod tests;

pub use opcodes::disassemble;
