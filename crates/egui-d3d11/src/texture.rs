use std::{mem::size_of, slice::from_raw_parts_mut};

use d3d11::{dxgi, D3D11_SUBRESOURCE_DATA};
use egui::{epaint::ahash::HashMap, Color32, ImageData, TextureId, TexturesDelta};

use crate::RenderError;

struct ManagedTexture {
    resource: d3d11::ShaderResourceView,
    texture: d3d11::Texture2D,
    pixels: Vec<Color32>,
    width: usize,
}

#[derive(Default)]
pub struct TextureAllocator {
    allocated: HashMap<TextureId, ManagedTexture>,
    /// User-loaded DX11 textures
    allocated_unmanaged:
        HashMap<TextureId, (d3d11::ShaderResourceView, Option<egui::TextureFilter>, bool)>,
    unmanaged_free_handles: Vec<TextureId>,
    unmanaged_index: u64,
    unmanaged_temporary_index: u64,
}

impl TextureAllocator {
    pub fn process_deltas(
        &mut self,
        dev: &d3d11::Device,
        ctx: &d3d11::DeviceContext,
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
    ) -> Option<(d3d11::ShaderResourceView, Option<egui::TextureFilter>, bool)> {
        self.allocated
            .get(&tid)
            .map(|t| (t.resource.clone(), None, true))
            .or_else(|| self.allocated_unmanaged.get(&tid).cloned())
    }

    pub fn allocate_dx(
        &mut self,
        srv: d3d11::ShaderResourceView,
        filter: Option<egui::TextureFilter>,
    ) -> TextureId {
        let tid = if let Some(t) = self.unmanaged_free_handles.pop() {
            t
        } else {
            self.unmanaged_index += 1;
            TextureId::User((1 << 60) + self.unmanaged_index)
        };
        self.allocated_unmanaged.insert(tid, (srv, filter, true));
        tid
    }

    /// Allocate a temporary texture that will be freed after the current frame finishes painting
    pub fn allocate_dx_temporary(
        &mut self,
        srv: d3d11::ShaderResourceView,
        filter: Option<egui::TextureFilter>,
        alpha: bool,
    ) -> TextureId {
        self.unmanaged_temporary_index += 1;
        let tid = TextureId::User((1 << 63) + self.unmanaged_temporary_index);

        self.allocated_unmanaged.insert(tid, (srv, filter, alpha));
        tid
    }

    pub fn clear_temporaries(&mut self) {
        self.unmanaged_temporary_index = 0;
        self.allocated_unmanaged.retain(|id, _| match id {
            TextureId::Managed(_) => true,
            TextureId::User(id) => *id < (1 << 63),
        });
    }

    pub fn set_filter(&mut self, tid: TextureId, filter: Option<egui::TextureFilter>) {
        if let Some((_, f, _)) = self.allocated_unmanaged.get_mut(&tid) {
            *f = filter;
        }
    }

    pub fn free(&mut self, tid: TextureId) -> bool {
        self.allocated
            .remove(&tid)
            .map(|_| ())
            .or_else(|| {
                let s = self.allocated_unmanaged.remove(&tid).map(|_| ());
                if s.is_some() {
                    self.unmanaged_free_handles.push(tid);
                }
                s
            })
            .is_some()
    }
}

impl TextureAllocator {
    fn allocate_new(
        &mut self,
        dev: &d3d11::Device,
        tid: TextureId,
        image: &ImageData,
    ) -> Result<(), RenderError> {
        let tex = Self::allocate_texture(dev, image)?;
        self.allocated.insert(tid, tex);
        Ok(())
    }

    fn update_partial(
        &mut self,
        ctx: &d3d11::DeviceContext,
        tid: TextureId,
        image: &ImageData,
        [nx, ny]: [usize; 2],
    ) -> Result<bool, RenderError> {
        if let Some(old) = self.allocated.get_mut(&tid) {
            let subr = ctx.map(&old.texture, 0, d3d11::MapType::WriteDiscard, false)?;

            match image {
                ImageData::Font(f) => unsafe {
                    let data: &mut [Color32] =
                        from_raw_parts_mut(subr.data as *mut Color32, old.pixels.len());
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

            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn allocate_texture(
        dev: &d3d11::Device,
        image: &ImageData,
    ) -> Result<ManagedTexture, RenderError> {
        let desc = d3d11::Texture2dDesc::builder()
            .width(image.width() as _)
            .height(image.height() as _)
            .mip_levels(1)
            .format(dxgi::Format::R8g8b8a8Unorm)
            .usage(d3d11::Usage::Dynamic)
            .bind_flags(d3d11::BindFlags::SHADER_RESOURCE)
            .cpu_access_flags(d3d11::CpuAccessFlags::WRITE)
            .build();

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
            SysMemPitch: (image.width() * size_of::<Color32>()) as u32,
            SysMemSlicePitch: 0,
        };

        let texture = dev.create_texture2d(&desc, Some(&[data]))?;

        let resource = dev.create_shader_resource_view(
            &texture,
            &d3d11::ShaderResourceViewDesc::builder()
                .format(dxgi::Format::R8g8b8a8Unorm)
                .view_dimension(d3d11::srv::SrvDimension::Texture2D {
                    most_detailed_mip: 0,
                    mip_levels: desc.mip_levels,
                })
                .build(),
        )?;

        Ok(ManagedTexture {
            width: image.width(),
            resource,
            pixels,
            texture,
        })
    }
}
