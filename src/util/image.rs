use std::{
    io::{Cursor, Read, Seek},
    sync::Arc,
};

use anyhow::Result;
use itertools::Itertools;
use png::ColorType;

pub struct Png {
    pub data: Arc<[u8]>,
    pub dimensions: [usize; 2],
    pub color_type: ColorType,
}

impl Png {
    /// Reads PNG data from a byte buffer
    /// When passing APNG data, only the first frame will be returned
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut reader = Cursor::new(data);
        Self::from_reader(&mut reader)
    }

    /// Reads PNG data from a reader
    /// When reading APNG data, only the first frame will be returned
    pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let decoder = png::Decoder::new(reader);
        let mut reader = decoder.read_info()?;
        let mut buf = vec![0; reader.output_buffer_size()];
        let frame_info = reader.next_frame(&mut buf)?;

        Ok(Self {
            data: buf.into(),
            dimensions: [frame_info.width as usize, frame_info.height as usize],
            color_type: frame_info.color_type,
        })
    }

    pub fn to_rgba(&self) -> Result<Self> {
        let new_png = Png {
            data: self.data.to_vec().into(),
            dimensions: self.dimensions,
            color_type: self.color_type,
        };

        new_png.into_rgba()
    }

    pub fn into_rgba(self) -> Result<Self> {
        match self.color_type {
            ColorType::Rgb => {
                let mut new_self = self;
                anyhow::ensure!(
                    new_self.data.len() % 3 == 0,
                    "Input data length must be a multiple of 3"
                );

                let new_data = new_self
                    .data
                    .chunks(3) // Split into RGB triplets
                    .flat_map(|rgb_triplet| {
                        let [r, g, b] = [rgb_triplet[0], rgb_triplet[1], rgb_triplet[2]];
                        vec![r, g, b, 255]
                    })
                    .collect_vec();

                new_self.data = new_data.into();

                Ok(new_self)
            }
            ColorType::Grayscale => {
                let mut new_self = self;

                let new_data = new_self
                    .data
                    .iter()
                    .flat_map(|&luminance| vec![luminance, luminance, luminance, 255])
                    .collect_vec();

                new_self.data = new_data.into();

                Ok(new_self)
            }
            ColorType::Rgba => Ok(self),
            c => todo!("Unsupported color conversion {c:?} -> RGBA"),
        }
    }

    // /// Converts RGBA data into PNG file data
    // pub fn from_rgba(data: &[u8], dimensions: (u32, u32)) -> Result<Vec<u8>> {
    //     let mut result = vec![];
    //     let mut encoder = png::Encoder::new(&mut result, dimensions.0, dimensions.1);
    //     encoder.set_color(ColorType::Rgba);
    //     encoder.set_depth(png::BitDepth::Eight);
    //     let mut writer = encoder.write_header()?;
    //     writer.write_image_data(data)?;
    //     writer.finish()?;
    //     Ok(result)
    // }
}
