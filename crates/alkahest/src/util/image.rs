use std::{mem::size_of, path::Path, sync::Arc};

use alkahest_renderer::util::image::Png;
use egui::{
    ahash::HashMap,
    load::{BytesPoll, ImagePoll, LoadError},
    mutex::Mutex,
    ColorImage,
};

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
