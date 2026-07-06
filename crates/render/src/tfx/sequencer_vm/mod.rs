use std::sync::{Arc, atomic::AtomicUsize};

use alkahest_data::tfx::sequencer::SExpression;
use glam::Vec4;
use tiger_parse::FnvHash;

pub mod global_channel;
pub mod interpreter;
pub mod opcodes;

#[derive(Clone)]
pub struct ObjectChannel {
    pub name: FnvHash,
    pub value: Vec4,
    pub expression: SExpression,
    pub usage: Arc<AtomicUsize>,
}
