use std::sync::atomic::{AtomicI8, Ordering};

use alkahest_data::tfx::common::AxisAlignedBBox;
use d3d11::dxgi;
use glam::{Mat4, UVec2, Vec2, Vec3, Vec4, vec2};

use crate::{Gpu, Renderer, camera::Camera, renderer::surface::SurfaceProxy};

#[derive(Debug)]
pub struct HzbLevel {
    pub width: u32,
    pub height: u32,
    pub data: Vec<f32>,
}

pub struct Hzb {
    pub first_mip: u32,
    pub levels: Vec<HzbLevel>,
    world_to_ndc: Mat4,
    camera_pos: Vec3,
    near: f32,
    far: f32,

    base_width: u32,
    base_height: u32,
}

impl Hzb {
    pub const EMPTY: Self = Self {
        first_mip: 0,
        levels: vec![],
        world_to_ndc: Mat4::IDENTITY,
        camera_pos: Vec3::ZERO,
        near: 0.0,
        far: 0.0,
        base_width: 0,
        base_height: 0,
    };

    #[profiling::function]
    pub fn download(gpu: &Gpu, surface: &SurfaceProxy, camera: &Camera) -> Self {
        assert!(
            surface.res.get_desc().format == dxgi::Format::R32Typeless,
            "HZB surface format must be R32Typeless"
        );
        const MAX_MIP_COUNT: u32 = 5;
        let (first_mip, mips) = if surface.mip_count() < MAX_MIP_COUNT {
            (0, surface.mip_count())
        } else {
            (surface.mip_count() - MAX_MIP_COUNT, MAX_MIP_COUNT)
        };

        let (width, height) = surface.res.get_desc().resolution();

        let mut levels = Vec::new();
        for i in first_mip..first_mip + mips {
            let map = gpu
                .context()
                .map(
                    &surface.res,
                    d3d11::calc_subresource_index(i, 0, mips),
                    d3d11::MapType::Read,
                    false,
                )
                .expect("Failed to map HZB surface mip");

            let width = (width >> i).max(1);
            let height = (height >> i).max(1);

            let mut data = vec![0f32; (width * height) as usize];
            for y in 0..height {
                for x in 0..width {
                    unsafe {
                        let v = *map
                            .data
                            .add((y * map.row_pitch + x * 4) as usize)
                            .cast::<f32>();
                        data[(y * width + x) as usize] = v;
                    }
                }
            }

            levels.push(HzbLevel {
                width,
                height,
                data,
            });
        }

        Self {
            first_mip,
            levels,
            world_to_ndc: camera.world_to_clip_space(),
            camera_pos: camera.position,
            near: camera.near,
            far: camera.far,
            base_width: width,
            base_height: height,
        }
    }

    fn linearize_depth(&self, depth: f32) -> f32 {
        self.near / depth
    }

    /// Returns the best mip level for the given NDC rect
    fn select_mip(&self, rect: &UvRect) -> usize {
        let base = &self.levels[0];
        let width = rect.width() * base.width as f32;
        let height = rect.height() * base.height as f32;
        let size = width.max(height).max(1e-5);

        let mip = size.log2().floor() as i32;
        mip.clamp(0, self.levels.len() as i32 - 1) as usize
    }

    pub fn project_aabb_to_screen(&self, aabb: &AxisAlignedBBox) -> Option<(UvRect, f32)> {
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut min_z = f32::NEG_INFINITY;

        let mut any_in_front = false;
        let mut any_behind = false;

        for c in aabb.points() {
            let clip = self.world_to_ndc * Vec4::new(c.x, c.y, c.z, 1.0);

            if clip.w <= 0.0 {
                any_behind = true;
                break;
            }

            any_in_front = true;
            let inv_w = 1.0 / clip.w;
            let ndc_x = clip.x * inv_w;
            let ndc_y = clip.y * inv_w;
            let ndc_z = clip.z * inv_w;

            // NDC -> viewport (eg [-1,1] to [0,1920])
            let sx = ndc_x * 0.5 + 0.5;
            let sy = -ndc_y * 0.5 + 0.5;

            min_x = min_x.min(sx);
            min_y = min_y.min(sy);
            max_x = max_x.max(sx);
            max_y = max_y.max(sy);

            min_z = min_z.max(ndc_z);
        }

        if !any_in_front {
            return None;
        }

        if any_behind {
            return Some((
                UvRect {
                    min: Vec2::ZERO,
                    max: Vec2::ONE,
                },
                0.0,
            ));
        }

        Some((
            UvRect {
                min: Vec2::new(min_x, min_y).clamp(Vec2::ZERO, Vec2::ONE),
                max: Vec2::new(max_x, max_y).clamp(Vec2::ZERO, Vec2::ONE),
            },
            min_z,
        ))
    }

    fn sample_hzb_min_4(&self, mip: usize, rect: &UvRect) -> f32 {
        let level = &self.levels[mip];

        let scale_x = level.width as f32;
        let scale_y = level.height as f32;

        let mut x0 = (rect.min.x * scale_x).floor() as i32;
        let mut y0 = (rect.min.y * scale_y).floor() as i32;
        let mut x1 = (rect.max.x * scale_x).ceil() as i32;
        let mut y1 = (rect.max.y * scale_y).ceil() as i32;

        let w = level.width as i32;
        let h = level.height as i32;
        x0 = x0.clamp(0, w - 1);
        y0 = y0.clamp(0, h - 1);
        x1 = x1.clamp(0, w - 1);
        y1 = y1.clamp(0, h - 1);

        // If the pixels have a gap inbetween, then this mip level is too small to sample, use a smaller one (with bigger pixels)
        if (x1 - x0) > 1 || (y1 - y0) > 1 {
            if (mip + 1) < self.levels.len() {
                return self.sample_hzb_min_4(mip + 1, rect);
            }
        }

        let idx =
            |x: i32, y: i32| -> usize { (y as usize) * (level.width as usize) + (x as usize) };

        let d00 = level.data[idx(x0, y0)];
        let d10 = level.data[idx(x1, y0)];
        let d01 = level.data[idx(x0, y1)];
        let d11 = level.data[idx(x1, y1)];

        d00.min(d10).min(d01).min(d11)
    }

    #[profiling::function]
    pub fn is_aabb_occluded(&self, aabb: &AxisAlignedBBox) -> bool {
        if self.levels.is_empty() {
            return false;
        }

        // Project to screen; if None, we treat the bb as visible.
        let (rect, z_near) = match self.project_aabb_to_screen(aabb) {
            Some(r) => r,
            None => return false, // visible
        };

        // Pick a mip where 4 texels cover the rect
        let mip = self.select_mip(&rect);

        let hzb_min = self.sample_hzb_min_4(mip, &rect);

        if z_near <= f32::EPSILON || self.linearize_depth(z_near) < 5.0 {
            return false;
        }

        (self.linearize_depth(z_near) - 0.0) > self.linearize_depth(hzb_min)
    }

    /// Helper for `is_aabb_occluded` to reduce mental gymnastics when checking visibility.
    pub fn is_aabb_visible(&self, aabb: &AxisAlignedBBox) -> bool {
        !self.is_aabb_occluded(aabb)
    }
}

// Screen-space rectangle in UV space
#[derive(Debug, Clone)]
pub struct UvRect {
    pub min: Vec2,
    pub max: Vec2,
}

impl UvRect {
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    pub fn center(&self) -> Vec2 {
        (self.min + self.max) / 2.0
    }
}

/// Gives occluded objects a period of grace before they are culled.
#[repr(transparent)]
pub struct CullingState(AtomicI8);

impl CullingState {
    pub const FRAMES: i8 = 4;

    pub const fn new() -> Self {
        Self(AtomicI8::new(Self::FRAMES))
    }

    pub fn is_visible(&self, camera: &Camera, aabb: &AxisAlignedBBox) -> bool {
        if !camera.is_visible(aabb) {
            return false;
        }

        let grace = if camera.hzb.is_aabb_occluded(aabb) {
            self.decrement()
        } else {
            self.reset()
        };

        grace > 0
    }

    fn reset(&self) -> i8 {
        self.0.store(Self::FRAMES, Ordering::Relaxed);
        Self::FRAMES
    }

    fn decrement(&self) -> i8 {
        let v = self.0.fetch_sub(1, Ordering::Relaxed);
        if v <= 0 {
            self.0.store(0, Ordering::Relaxed);
        }
        v.max(0)
    }
}

impl Default for CullingState {
    fn default() -> Self {
        Self::new()
    }
}
