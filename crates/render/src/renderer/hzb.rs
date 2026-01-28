use alkahest_data::tfx::common::AxisAlignedBBox;
use d3d11::dxgi;
use glam::{Mat4, Vec2, Vec4};

use crate::{Gpu, camera::Camera, renderer::surface::SurfaceProxy};

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
    near: f32,
    _far: f32,

    base_width: u32,
    base_height: u32,
}

impl Hzb {
    pub const MAX_MIP_COUNT: u32 = 6;

    pub const EMPTY: Self = Self {
        first_mip: 0,
        levels: vec![],
        world_to_ndc: Mat4::IDENTITY,
        near: 0.0,
        _far: 0.0,
        base_width: 0,
        base_height: 0,
    };

    #[profiling::function]
    pub fn download(gpu: &Gpu, surface: &SurfaceProxy, camera: &Camera) -> Self {
        assert!(
            surface.res.get_desc().format == dxgi::Format::R32Typeless,
            "HZB surface format must be R32Typeless"
        );
        let (first_mip, mips) = if surface.mip_count() < Self::MAX_MIP_COUNT {
            (0, surface.mip_count())
        } else {
            (
                surface.mip_count() - Self::MAX_MIP_COUNT,
                Self::MAX_MIP_COUNT,
            )
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

        // for (mip, level) in levels.iter().enumerate() {
        //     println!("mip {}: {}x{}", mip, level.width, level.height);
        //     let z_min = level
        //         .data
        //         .iter()
        //         .copied()
        //         .reduce(|a, b| a.min(b))
        //         .unwrap_or_default();

        //     let z_max = level
        //         .data
        //         .iter()
        //         .copied()
        //         .reduce(|a, b| a.max(b))
        //         .unwrap_or_default();

        //     println!("Min {z_min} Max {z_max}");
        //     for (i, value) in level.data.iter().enumerate() {
        //         if (i % level.width as usize) == 0 {
        //             println!();
        //         }

        //         let v = (*value - z_min) / (z_max - z_min);
        //         // 24-bit ANSI display
        //         let v_clamped = if v.is_finite() {
        //             v.clamp(0.0, 1.0)
        //         } else {
        //             0.0
        //         };

        //         let lum = (v_clamped * 255.0) as u8;

        //         // Print two full block characters colored with 24-bit ANSI (foreground)
        //         print!("\x1b[38;2;{};{};{}m██", lum, lum, lum);
        //     }
        //     println!("\x1b[0m");
        // }

        Self {
            first_mip,
            levels,
            world_to_ndc: camera.world_to_clip_space(),
            near: camera.near,
            _far: camera.far,
            base_width: width,
            base_height: height,
        }
    }

    fn linearize_depth(&self, depth: f32) -> f32 {
        self.near / depth
    }

    /// Returns the best mip level for the given NDC rect
    fn select_mip(&self, rect: &UvRect) -> usize {
        let rect_px = (rect.width() * self.base_width as f32)
            .max(rect.height() * self.base_height as f32)
            .max(1.0);

        let mip = (rect_px.log2()).ceil() as i32 - self.first_mip as i32;
        mip.clamp(0, self.levels.len() as i32 - 1) as usize
    }

    pub fn project_aabb_to_screen(&self, aabb: &AxisAlignedBBox) -> Option<(UvRect, f32)> {
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut nearest_z = f32::NEG_INFINITY;

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

            nearest_z = nearest_z.max(ndc_z);
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
                1.0,
            ));
        }

        Some((
            UvRect {
                min: Vec2::new(min_x, min_y).clamp(Vec2::ZERO, Vec2::ONE),
                max: Vec2::new(max_x, max_y).clamp(Vec2::ZERO, Vec2::ONE),
            },
            nearest_z,
        ))
    }

    fn sample_hzb_min_4(&self, mip: usize, rect: &UvRect) -> f32 {
        let level = &self.levels[mip];

        let x0_base = (rect.min.x * self.base_width as f32).floor() as i32;
        let y0_base = (rect.min.y * self.base_height as f32).floor() as i32;

        let x1_base = ((rect.max.x * self.base_width as f32) - 1e-6).floor() as i32;
        let y1_base = ((rect.max.y * self.base_height as f32) - 1e-6).floor() as i32;

        let absolute_shift = mip + self.first_mip as usize;

        let x0 = (x0_base >> absolute_shift).clamp(0, (level.width - 1) as i32);
        let y0 = (y0_base >> absolute_shift).clamp(0, (level.height - 1) as i32);
        let x1 = (x1_base >> absolute_shift).clamp(0, (level.width - 1) as i32);
        let y1 = (y1_base >> absolute_shift).clamp(0, (level.height - 1) as i32);
        // 3. Perform the conservative 2x2 min sample
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

        let z_near_linear = self.linearize_depth(z_near);

        if z_near_linear < 1.0 {
            return false;
        }

        // Pick the coarsest mip where the rect fits within 2x2 texels
        let mip = self.select_mip(&rect);

        let hzb_min = self.sample_hzb_min_4(mip, &rect);

        z_near_linear > self.linearize_depth(hzb_min)
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

// /// Gives occluded objects a period of grace before they are culled.
// #[repr(transparent)]
// pub struct CullingState(AtomicI8);

// impl CullingState {
//     pub const FRAMES: i8 = 4;

//     pub const fn new() -> Self {
//         Self(AtomicI8::new(Self::FRAMES))
//     }

//     pub fn is_visible(&self, camera: &Camera, aabb: &AxisAlignedBBox) -> bool {
//         if !camera.is_visible(aabb) {
//             return false;
//         }

//         let grace = if camera.hzb.is_aabb_occluded(aabb) {
//             self.decrement()
//         } else {
//             self.reset()
//         };

//         grace > 0
//     }

//     fn reset(&self) -> i8 {
//         self.0.store(Self::FRAMES, Ordering::Relaxed);
//         Self::FRAMES
//     }

//     fn decrement(&self) -> i8 {
//         let v = self.0.fetch_sub(1, Ordering::Relaxed);
//         if v <= 0 {
//             self.0.store(0, Ordering::Relaxed);
//         }
//         v.max(0)
//     }
// }

// impl Default for CullingState {
//     fn default() -> Self {
//         Self::new()
//     }
// }
