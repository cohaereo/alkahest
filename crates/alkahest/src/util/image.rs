use std::{
    io::{Cursor, Read, Seek},
    mem::size_of,
    path::Path,
    sync::Arc,
};

use anyhow::Result;
use egui::{
    ahash::HashMap,
    load::{BytesPoll, ImagePoll, LoadError},
    mutex::Mutex,
    ColorImage,
};
use itertools::Itertools;
use png::{BitDepth, ColorType};

pub struct Png {
    pub data: Arc<[u8]>,
    pub dimensions: [usize; 2],
    pub color_type: ColorType,
    pub bit_depth: BitDepth,
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
            bit_depth: frame_info.bit_depth,
        })
    }

    pub fn to_rgba(&self) -> Result<Self> {
        let new_png = Png {
            data: self.data.to_vec().into(),
            dimensions: self.dimensions,
            color_type: self.color_type,
            bit_depth: self.bit_depth,
        };

        new_png.into_rgba()
    }

    pub fn into_rgba(self) -> Result<Self> {
        match self.bit_depth {
            BitDepth::Eight => self.into_rgba_impl::<u8>(),
            // BitDepth::Sixteen => self.into_rgba_impl::<u16>(),
            u => todo!("into_rgba: Unsupported PNG bit depth {u:?}"),
        }
    }

    fn into_rgba_impl<T: num::Bounded + num::ToPrimitive + bytemuck::Pod + Sized>(
        self,
    ) -> Result<Self> {
        let num_size = std::mem::size_of::<T>();
        let num_max = T::max_value();

        match self.color_type {
            ColorType::Rgb => {
                let mut new_self = self;
                anyhow::ensure!(
                    new_self.data.len() % (3 * num_size) == 0,
                    "Input data length must be a multiple of {} (3 * {})",
                    3 * num_size,
                    num_size
                );

                let new_data = bytemuck::cast_slice::<u8, T>(&new_self.data)
                    .chunks_exact(3) // Split into RGB triplets
                    .flat_map(|rgb_triplet| {
                        let [r, g, b] = [rgb_triplet[0], rgb_triplet[1], rgb_triplet[2]];
                        [r, g, b, num_max]
                    })
                    .collect_vec();

                // TODO(cohae): Another conversion seems excessive
                new_self.data = bytemuck::cast_slice::<T, u8>(&new_data).into();

                Ok(new_self)
            }
            ColorType::Grayscale => {
                let mut new_self = self;

                let new_data = bytemuck::cast_slice::<u8, T>(&new_self.data)
                    .iter()
                    .flat_map(|&luminance| [luminance, luminance, luminance, num_max])
                    .collect_vec();

                // TODO(cohae): Another conversion seems excessive
                new_self.data = bytemuck::cast_slice::<T, u8>(&new_data).into();

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

type ImageCacheEntry = Result<Arc<ColorImage>, String>;

#[derive(Default)]
pub struct EguiPngLoader {
    cache: Mutex<HashMap<String, ImageCacheEntry>>,
}

fn is_supported(uri: &str) -> bool {
    let Some(ext) = Path::new(uri).extension().and_then(|ext| ext.to_str()) else {
        return false;
    };

    ext == "png"
}

fn load_png_bytes(bytes: &[u8]) -> Result<ColorImage, String> {
    let png = Png::from_bytes(bytes).map_err(|err| err.to_string())?;
    let png = png.into_rgba().map_err(|err| err.to_string())?;
    Ok(ColorImage::from_rgba_unmultiplied(
        png.dimensions,
        &png.data,
    ))
}

impl egui::load::ImageLoader for EguiPngLoader {
    fn id(&self) -> &str {
        egui::generate_loader_id!(EguiPngLoader)
    }

    fn load(
        &self,
        ctx: &egui::Context,
        uri: &str,
        _size_hint: egui::SizeHint,
    ) -> egui::load::ImageLoadResult {
        if !is_supported(uri) {
            return Err(LoadError::NotSupported);
        }

        let mut cache = self.cache.lock();
        if let Some(entry) = cache.get(uri).cloned() {
            match entry {
                Ok(image) => Ok(ImagePoll::Ready { image }),
                Err(err) => Err(LoadError::Loading(err)),
            }
        } else {
            match ctx.try_load_bytes(uri) {
                Ok(BytesPoll::Ready { bytes, .. }) => {
                    let result = load_png_bytes(&bytes).map(Arc::new);
                    cache.insert(uri.into(), result.clone());
                    match result {
                        Ok(image) => Ok(ImagePoll::Ready { image }),
                        Err(err) => Err(LoadError::Loading(err)),
                    }
                }
                Ok(BytesPoll::Pending { size }) => Ok(ImagePoll::Pending { size }),
                Err(err) => Err(err),
            }
        }
    }

    fn forget(&self, uri: &str) {
        _ = self.cache.lock().remove(uri);
    }

    fn forget_all(&self) {
        self.cache.lock().clear();
    }

    fn byte_size(&self) -> usize {
        self.cache
            .lock()
            .values()
            .map(|result| match result {
                Ok(image) => image.pixels.len() * size_of::<egui::Color32>(),
                Err(err) => err.len(),
            })
            .sum()
    }
}
