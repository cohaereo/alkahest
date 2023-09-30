use std::{
    io::{Cursor, Read, Seek},
    sync::Arc,
};

use anyhow::Result;

pub struct Png {
    pub data: Arc<[u8]>,
    pub dimensions: [usize; 2],
}

impl Png {
    /// Reads PNG data from a byte buffer
    /// When passing APNG data, only the first frame will be returned
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut reader = Cursor::new(data);
        Self::from_reader(&mut reader)
    }

    /// Reads PNG data from a byte buffer
    /// When passing APNG data, only the first frame will be returned
    pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let decoder = png::Decoder::new(reader);
        let mut reader = decoder.read_info()?;
        let mut buf = vec![0; reader.output_buffer_size()];
        let frame_info = reader.next_frame(&mut buf)?;

        Ok(Self {
            data: buf.into(),
            dimensions: [frame_info.width as usize, frame_info.height as usize],
        })
    }
}
