use std::env;
use std::fs::File;
use bitflags::bitflags;
use std::io::prelude::*;

use crate::vertex_layout::InputElement;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct TypeFlags: u8 {
        const VERTEX_FORMAT = 0b00000001;
    }
}

pub struct DataAggregator {
    pub vertex_formats: Vec<Vec<InputElement>>
}

impl DataAggregator {

    pub fn vertex_format(&mut self, inputs: Vec<InputElement>) {
        if !self.vertex_formats.contains(&inputs) {
            self.vertex_formats.push(inputs.clone());
        }
    }

    pub fn dump(&mut self, data_types: TypeFlags) {
        let path = env::current_dir().unwrap();
        if data_types == TypeFlags::VERTEX_FORMAT {
            let mut file = File::create(path.join("vertex_format.md")).unwrap();
            for inputs in &mut self.vertex_formats {
                writeln!(&mut file, "| Register | Parameter Name | DX Type |").unwrap();
                writeln!(&mut file, "|:--------:|:--------------:|:--------|").unwrap();
                for (index, input) in inputs.iter().enumerate() {
                    unsafe {
                        writeln!(&mut file, "| v{0} | {1}{2} | {3:?} |", index, input.semantic_type.to_pcstr().to_string().unwrap(), input.semantic_index, input.format).unwrap();
                    }
                }
                writeln!(&mut file).unwrap();
            }
        }
    } 
}