use egui::{epaint::ahash::HashMap, Color32, ImageData, TextureId, TexturesDelta};
use std::{mem::size_of, slice::from_raw_parts_mut};
use windows::Win32::Graphics::{
    Direct3D::D3D11_SRV_DIMENSION_TEXTURE2D,
    Direct3D11::{
        ID3D11Device, ID3D11DeviceContext, ID3D11ShaderResourceView, ID3D11Texture2D,
        D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_WRITE, D3D11_MAPPED_SUBRESOURCE,
        D3D11_MAP_WRITE_DISCARD, D3D11_SHADER_RESOURCE_VIEW_DESC,
        D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_SUBRESOURCE_DATA, D3D11_TEX2D_SRV,
        D3D11_TEXTURE2D_DESC, D3D11_USAGE_DYNAMIC,
    },
    Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC},
};

use crate::RenderError;

struct ManagedTexture {
    resource: ID3D11ShaderResourceView,
    texture: ID3D11Texture2D,
    pixels: Vec<Color32>,
    width: usize,
}

#[derive(Default)]
pub struct TextureAllocator {
    allocated: HashMap<TextureId, ManagedTexture>,
    /// User-loaded DX11 textures
    allocated_unmanaged:
        HashMap<TextureId, (ID3D11ShaderResourceView, Option<egui::TextureFilter>)>,
    unmanaged_index: u64,
}

impl TextureAllocator {
    pub fn process_deltas(
        &mut self,
        dev: &ID3D11Device,
        ctx: &ID3D11DeviceContext,
        delta: &TexturesDelta,
    ) -> Result<(), RenderError> {
        for (tid, delta) in &delta.set {
            if delta.is_whole() {
                self.allocate_new(dev, *tid, &delta.image)?;
            } else {
                let _did_update =
                    self.update_partial(ctx, *tid, &delta.image, delta.pos.unwrap())?;
            }
        }

        for tid in &delta.free {
            self.free(*tid);
        }

        Ok(())
    }

    pub fn get_by_id(
        &self,
        tid: TextureId,
    ) -> Option<(ID3D11ShaderResourceView, Option<egui::TextureFilter>)> {
        self.allocated
            .get(&tid)
            .map(|t| (t.resource.clone(), None))
            .or_else(|| self.allocated_unmanaged.get(&tid).cloned())
    }

    pub fn allocate_dx(
        &mut self,
        srv: (ID3D11ShaderResourceView, Option<egui::TextureFilter>),
    ) -> TextureId {
        self.unmanaged_index += 1;
        let tid = TextureId::User((1 << 60) + self.unmanaged_index);
        self.allocated_unmanaged.insert(tid, srv);
        tid
    }

    pub fn set_filter(&mut self, tid: TextureId, filter: Option<egui::TextureFilter>) {
        if let Some((_, f)) = self.allocated_unmanaged.get_mut(&tid) {
            *f = filter;
        }
    }

    pub fn free(&mut self, tid: TextureId) -> bool {
        self.allocated
            .remove(&tid)
            .map(|_| ())
            .or_else(|| self.allocated_unmanaged.remove(&tid).map(|_| ()))
            .is_some()
    }
}

impl TextureAllocator {
    fn allocate_new(
        &mut self,
        dev: &ID3D11Device,
        tid: TextureId,
        image: &ImageData,
    ) -> Result<(), RenderError> {
        let tex = Self::allocate_texture(dev, image)?;
        self.allocated.insert(tid, tex);
        Ok(())
    }

    fn update_partial(
        &mut self,
        ctx: &ID3D11DeviceContext,
        tid: TextureId,
        image: &ImageData,
        [nx, ny]: [usize; 2],
    ) -> Result<bool, RenderError> {
        if let Some(old) = self.allocated.get_mut(&tid) {
            let subr = unsafe {
                let mut output = D3D11_MAPPED_SUBRESOURCE::default();
                ctx.Map(
                    &old.texture,
                    0,
                    D3D11_MAP_WRITE_DISCARD,
                    0,
                    Some(&mut output),
                )?;
                output
            };

            match image {
                ImageData::Font(f) => unsafe {
                    let data = from_raw_parts_mut(subr.pData as *mut Color32, old.pixels.len());
                    data.as_mut_ptr()
                        .copy_from_nonoverlapping(old.pixels.as_ptr(), old.pixels.len());

                    let new: Vec<Color32> = f
                        .pixels
                        .iter()
                        .map(|a| Color32::from_rgba_premultiplied(255, 255, 255, (a * 255.) as u8))
                        .collect();

                    for y in 0..f.height() {
                        for x in 0..f.width() {
                            let whole = (ny + y) * old.width + nx + x;
                            let frac = y * f.width() + x;
                            old.pixels[whole] = new[frac];
                            data[whole] = new[frac];
                        }
                    }
                },
                _ => unreachable!(),
            }

            unsafe {
                ctx.Unmap(&old.texture, 0);
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn allocate_texture(
        dev: &ID3D11Device,
        image: &ImageData,
    ) -> Result<ManagedTexture, RenderError> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: image.width() as _,
            Height: image.height() as _,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DYNAMIC,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
            ..Default::default()
        };

        // rust is cringe sometimes
        let width = image.width();
        let pixels = match image {
            ImageData::Color(c) => c.pixels.clone(),
            ImageData::Font(f) => f
                .pixels
                .iter()
                .map(|a| Color32::from_rgba_premultiplied(255, 255, 255, (a * 255.) as u8))
                .collect(),
        };

        let data = D3D11_SUBRESOURCE_DATA {
            pSysMem: pixels.as_ptr() as _,
            SysMemPitch: (width * size_of::<Color32>()) as u32,
            SysMemSlicePitch: 0,
        };

        unsafe {
            let texture = {
                let mut output_texture = None;
                dev.CreateTexture2D(&desc, Some(&data), Some(&mut output_texture))?;
                output_texture.ok_or(RenderError::General("Unable to create Texture 2D"))?
            };

            let desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                    Texture2D: D3D11_TEX2D_SRV {
                        MostDetailedMip: 0,
                        MipLevels: desc.MipLevels,
                    },
                },
            };

            let resource = {
                let mut output_resource = None;
                dev.CreateShaderResourceView(&texture, Some(&desc), Some(&mut output_resource))?;
                output_resource.ok_or(RenderError::General("Failed to create shader view"))?
            };

            Ok(ManagedTexture {
                width,
                resource,
                pixels,
                texture,
            })
        }
    }
}
