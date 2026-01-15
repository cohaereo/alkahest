use std::{
    cell::UnsafeCell,
    fmt::{Display, Formatter},
};

use anyhow::Context;
use bon::Builder;
use crossbeam::atomic::AtomicCell;
use d3d11::{
    BindFlags, ClearFlags, CpuAccessFlags, DepthStencilViewDesc, DeviceChild, RenderTargetViewDesc,
    ShaderResourceViewDesc, Texture2dDesc, UnorderedAccessViewDesc, dsv::DsvFlags, dxgi,
    srv::SrvDimension, uav::UavDimension,
};
use glam::Vec4;
use smallvec::SmallVec;

use super::Renderer;
use crate::gpu::command_list::CommandList;

#[derive(Clone, Copy, Debug)]
pub struct SurfaceHandle(u32);

impl SurfaceHandle {
    pub fn index(&self) -> u32 {
        self.0
    }
}

// TODO(cohae): Needs to be reworked, preferably a la AssetManager where we have ArcShift handles
pub struct Surfaces {
    surfaces: UnsafeCell<Vec<Surface>>,
    device: d3d11::Device,
    current_swapchain_resolution: AtomicCell<(u32, u32)>,
    resolution_scale: AtomicCell<f32>,
}

unsafe impl Sync for Surfaces {}

impl Surfaces {
    pub fn new(device: d3d11::Device, swapchain_resolution: (u32, u32)) -> Self {
        Self {
            surfaces: UnsafeCell::new(Vec::new()),
            device,
            resolution_scale: AtomicCell::new(1.0),
            current_swapchain_resolution: AtomicCell::new(swapchain_resolution),
        }
    }

    #[allow(clippy::mut_from_ref)]
    unsafe fn surfaces_mut(&self) -> &mut Vec<Surface> {
        &mut *self.surfaces.get()
    }

    fn surfaces(&self) -> &[Surface] {
        unsafe { &*self.surfaces.get() }
    }

    pub fn iter(&self) -> impl Iterator<Item = (SurfaceHandle, &Surface)> {
        self.surfaces()
            .iter()
            .enumerate()
            .map(|(i, s)| (SurfaceHandle(i as u32), s))
    }

    pub fn create_surface(
        &self,
        base_resolution: (u32, u32),
        desc: SurfaceDesc,
    ) -> anyhow::Result<SurfaceHandle> {
        let surface = Surface::new(&self.device, base_resolution, desc.clone())
            .with_context(|| format!("Failed to create surface '{}'", desc.label))?;
        let handle = SurfaceHandle(self.surfaces().len() as u32);
        unsafe { self.surfaces_mut().push(surface) };
        Ok(handle)
    }

    pub fn get(&self, handle: SurfaceHandle) -> &Surface {
        &self.surfaces()[handle.0 as usize]
    }

    pub fn get_by_index(&self, index: usize) -> Option<&Surface> {
        self.surfaces().get(index)
    }

    pub fn resize_surfaces(&self, swapchain_resolution: (u32, u32)) {
        if swapchain_resolution == self.swapchain_resolution() {
            return;
        }
        self.current_swapchain_resolution
            .store(swapchain_resolution);

        self.refresh_surfaces();
    }

    /// Recreates all relatively sized surface
    fn refresh_surfaces(&self) {
        for surface in unsafe { self.surfaces_mut() }
            .iter_mut()
            .filter(|s| s.desc.size_relativity != SizeRelativity::Absolute)
        {
            let base_resolution = match surface.desc.size_relativity {
                SizeRelativity::RelativeToFramebuffer => self.framebuffer_resolution(),
                SizeRelativity::RelativeToSwapchain => self.swapchain_resolution(),
                _ => unreachable!(),
            };
            surface.resize(&self.device, base_resolution);
        }
    }

    pub fn swapchain_resolution(&self) -> (u32, u32) {
        self.current_swapchain_resolution.load()
    }

    pub fn framebuffer_resolution(&self) -> (u32, u32) {
        let swapchain_resolution = self.swapchain_resolution();
        let scale = self.resolution_scale.load();
        (
            (swapchain_resolution.0 as f32 * scale).ceil() as u32,
            (swapchain_resolution.1 as f32 * scale).ceil() as u32,
        )
    }

    /// Convenience function to copy the contents of one surface to another.
    /// This is unchecked, and will silently fail if the surfaces are incompatible (check d3d11 CopyResource documentation)
    pub fn copy(&self, cmd: &mut CommandList, src: SurfaceHandle, dst: SurfaceHandle) {
        let src = self.get(src);
        let dst = self.get(dst);
        cmd.copy_resource(&src.texture, &dst.texture);
    }

    pub fn set_resolution_scale(&self, scale: f32) {
        let old = self.resolution_scale.swap(scale);
        if old != scale {
            self.refresh_surfaces();
        }
    }
}

pub struct Surface {
    desc: SurfaceDesc,
    current_base_resolution: (u32, u32),
    pub texture: d3d11::Texture2D,
    pub srv: Option<d3d11::ShaderResourceView>,
    pub rtv: Option<d3d11::RenderTargetView>,
    pub dsv: Option<d3d11::DepthStencilView>,
    pub uav: Option<d3d11::UnorderedAccessView>,
}

impl Surface {
    pub fn new(
        device: &d3d11::Device,
        base_resolution: (u32, u32),
        desc: SurfaceDesc,
    ) -> anyhow::Result<Self> {
        let (width, height) = desc.scale.scale_resolution(base_resolution);

        let depth_format = desc.depth_format.unwrap_or(desc.format);
        let mut bind_flags = if depth_format.is_depth() {
            BindFlags::DEPTH_STENCIL
        } else {
            BindFlags::RENDER_TARGET
        };

        if desc.create_uav {
            bind_flags |= BindFlags::UNORDERED_ACCESS;
        }

        let texture = device
            .create_texture2d(
                &Texture2dDesc::builder()
                    .width(width)
                    .height(height)
                    .mip_levels(1)
                    .format(desc.format)
                    .bind_flags(bind_flags | BindFlags::SHADER_RESOURCE)
                    .build(),
                None,
            )
            .context("Failed to create surface texture")?;

        let mut srv = None;
        let mut uav = None;

        if !desc.view_format.is_depth() {
            let r = device
                .create_shader_resource_view(
                    &texture,
                    &ShaderResourceViewDesc::builder()
                        .format(desc.view_format)
                        .view_dimension(SrvDimension::Texture2D {
                            mip_levels: 1,
                            most_detailed_mip: 0,
                        })
                        .build(),
                )
                .context("Failed to create surface SRV")?;

            r.set_debug_name(format!("{} (View)", &desc.label));
            srv = Some(r);
        }

        if desc.create_uav {
            let r = device
                .create_unordered_access_view(
                    &texture,
                    &UnorderedAccessViewDesc::builder()
                        .format(desc.view_format)
                        .view_dimension(UavDimension::Texture2D { mip_slice: 0 })
                        .build(),
                )
                .context("Failed to create surface UAV")?;

            r.set_debug_name(format!("{} (UAV)", &desc.label));
            uav = Some(r);
        }

        let mut rtv = None;
        let mut dsv = None;

        if depth_format.is_depth() {
            let r = device
                .create_depth_stencil_view(
                    &texture,
                    Some(
                        &DepthStencilViewDesc::builder()
                            .format(depth_format)
                            .view_dimension(d3d11::dsv::DsvDimension::Texture2D {
                                flags: DsvFlags::empty(),
                                mip_slice: 0,
                            })
                            .build(),
                    ),
                )
                .context("Failed to create surface DSV")?;
            r.set_debug_name(format!("{} (DSV)", &desc.label));
            dsv = Some(r);
        } else {
            let r = device
                .create_render_target_view(
                    &texture,
                    Some(
                        &RenderTargetViewDesc::builder()
                            .format(desc.view_format)
                            .view_dimension(d3d11::rtv::RtvDimension::Texture2D { mip_slice: 0 })
                            .build(),
                    ),
                )
                .context("Failed to create surface RTV")?;
            r.set_debug_name(format!("{} (RT)", &desc.label));
            rtv = Some(r);
        }

        texture.set_debug_name(&desc.label);

        Ok(Self {
            desc,
            current_base_resolution: base_resolution,
            texture,
            srv,
            rtv,
            dsv,
            uav,
        })
    }

    pub fn name(&self) -> &str {
        &self.desc.label
    }

    pub fn desc(&self) -> &SurfaceDesc {
        &self.desc
    }

    pub fn viewport(&self) -> d3d11::Viewport {
        let (width, height) = self.texture.get_desc().resolution();
        d3d11::Viewport {
            top_left_x: 0.0,
            top_left_y: 0.0,
            width: width as f32,
            height: height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }

    /// Binds only this surface's RTV or DSV and nothing else
    pub fn bind_single(&self, cmd: &mut CommandList) {
        cmd.rasterizer_set_viewports(&[self.viewport()]);
        if let Some(rtv) = &self.rtv {
            cmd.output_merger_set_render_targets(&[Some(rtv)], None);
        } else if let Some(dsv) = &self.dsv {
            cmd.output_merger_set_render_targets(&[], Some(dsv));
        }
    }

    pub fn resize(&mut self, device: &d3d11::Device, base_resolution: (u32, u32)) {
        // Avoid unnecessary resizing
        if base_resolution == self.current_base_resolution {
            return;
        }

        *self = Self::new(device, base_resolution, self.desc.clone())
            .expect("Failed to resize surface");
    }

    pub fn clear_depth(&self, context: &d3d11::DeviceContext, clear_value: f32, stencil_ref: u8) {
        if let Some(dsv) = &self.dsv {
            context.clear_depth_stencil_view(dsv, ClearFlags::all(), clear_value, stencil_ref);
        }
    }

    pub fn clear_color(&self, context: &d3d11::DeviceContext, color: [f32; 4]) {
        if let Some(rtv) = &self.rtv {
            context.clear_render_target_view(rtv, &color);
        }
    }

    pub fn resolution(&self) -> (u32, u32) {
        self.texture.get_desc().resolution()
    }

    pub fn resolution_with_recip(&self) -> Vec4 {
        let (width, height) = self.resolution();
        Vec4::new(
            width as f32,
            height as f32,
            1.0 / width as f32,
            1.0 / height as f32,
        )
    }
}

#[derive(Builder, Clone)]
pub struct SurfaceDesc {
    #[builder(start_fn, into)]
    pub label: String,

    /// Determines if this surface's resolution should be relative to the framebuffer's resolution.
    /// If true, calling [`SurfaceManager::resize_surfaces`] will resize this surface relative to the main view's resolution.
    #[builder(start_fn)]
    pub size_relativity: SizeRelativity,

    #[builder(default)]
    pub scale: SurfaceScale,

    pub format: dxgi::Format,
    pub depth_format: Option<dxgi::Format>,
    #[builder(default = format)]
    pub view_format: dxgi::Format,

    #[builder(default = false)]
    create_uav: bool,
}

impl SurfaceDesc {
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.label = name.into();
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SizeRelativity {
    /// Resize is not called automatically
    Absolute,
    /// Resize is called with the base resolution of the framebuffer (which includes resolution scaling)
    RelativeToFramebuffer,
    /// Resize is called with the base resolution of the swapchain
    RelativeToSwapchain,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum SurfaceScale {
    /// 1:1 scale (1920x1080)
    #[default]
    Full,
    /// 1:2 scale (1920x1080 => 960x540)
    Half,
    /// 1:3 scale (1920x1080 => 640x360)
    Third,
    /// 1:4 scale (1920x1080 => 480x270)
    Quarter,
    /// 1:6 scale (1920x1080 => 320x180)
    Sixth,
    /// 1:8 scale (1920x1080 => 240x135)
    Eighth,
    /// 1:12 scale (1920x1080 => 160x90)
    Twelfth,
    /// 1:16 scale (1920x1080 => 120x67)
    Sixteenth,
    /// 1:24 scale (1920x1080 => 80x45)
    TwentyFourth,
    /// Custom scale fraction
    Nth(f32, f32),
}

impl SurfaceScale {
    pub fn even(fraction: f32) -> Self {
        Self::Nth(fraction, fraction)
    }

    pub fn factor(&self) -> (f32, f32) {
        match self {
            Self::Full => (1.0, 1.0),
            Self::Half => (2.0, 2.0),
            Self::Third => (3.0, 3.0),
            Self::Sixth => (6.0, 6.0),
            Self::Quarter => (4.0, 4.0),
            Self::Twelfth => (12.0, 12.0),
            Self::Eighth => (8.0, 8.0),
            Self::Sixteenth => (16.0, 16.0),
            Self::TwentyFourth => (24.0, 24.0),
            Self::Nth(x, y) => (*x, *y),
        }
    }

    pub fn scale_resolution(&self, base_resolution: (u32, u32)) -> (u32, u32) {
        let (factor_x, factor_y) = self.factor();
        (
            if factor_x == 0.0 {
                1
            } else {
                (base_resolution.0 as f32 / factor_x).ceil() as u32
            },
            if factor_y == 0.0 {
                1
            } else {
                (base_resolution.1 as f32 / factor_y).ceil() as u32
            },
        )
    }
}

impl Display for SurfaceScale {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => write!(f, "1/1"),
            Self::Half => write!(f, "1/2"),
            Self::Third => write!(f, "1/3"),
            Self::Quarter => write!(f, "1/4"),
            Self::Sixth => write!(f, "1/6"),
            Self::Eighth => write!(f, "1/8"),
            Self::Twelfth => write!(f, "1/12"),
            Self::Sixteenth => write!(f, "1/16"),
            Self::TwentyFourth => write!(f, "1/24"),
            Self::Nth(x, y) => write!(f, "1/{x}, 1/{y}"),
        }
    }
}

/// A copy of a surface that can be used as SRV, not managed by the SurfaceManager, but resized on demand
pub struct SurfaceProxy {
    pub res: d3d11::Texture2D,
    pub srv: d3d11::ShaderResourceView,
    view_format: Option<dxgi::Format>,
    cpu_read: bool,
}

impl SurfaceProxy {
    pub fn new(
        device: &d3d11::Device,
        surface: &Surface,
        view_format: Option<dxgi::Format>,
        cpu_read: bool,
    ) -> anyhow::Result<Self> {
        let desc = surface.texture.get_desc();
        let res = device
            .create_texture2d(
                &Texture2dDesc::builder()
                    .width(desc.width)
                    .height(desc.height)
                    .mip_levels(1)
                    .format(surface.desc.format)
                    .bind_flags(if cpu_read {
                        BindFlags::empty()
                    } else {
                        BindFlags::SHADER_RESOURCE
                    })
                    .cpu_access_flags(if cpu_read {
                        CpuAccessFlags::READ
                    } else {
                        CpuAccessFlags::empty()
                    })
                    .usage(if cpu_read {
                        d3d11::Usage::Staging
                    } else {
                        d3d11::Usage::Default
                    })
                    .build(),
                None,
            )
            .context("Failed to create surface proxy texture")?;

        res.set_debug_name(format!("{} proxy", surface.name()));

        // cohae: Hack so we can use SurfaceProxy as CPU read buffers without having to make the SRV optional
        let srv = if cpu_read {
            let dummy_tex = device.create_texture2d(
                &Texture2dDesc::builder()
                    .width(2)
                    .height(2)
                    .mip_levels(1)
                    .format(dxgi::Format::R8g8b8a8Unorm)
                    .bind_flags(BindFlags::SHADER_RESOURCE)
                    .cpu_access_flags(CpuAccessFlags::empty())
                    .build(),
                None,
            )?;
            device.create_shader_resource_view(&dummy_tex, None)
        } else {
            device.create_shader_resource_view(
                &res,
                Some(
                    &ShaderResourceViewDesc::builder()
                        .format(view_format.unwrap_or(surface.desc.view_format))
                        .view_dimension(SrvDimension::Texture2D {
                            mip_levels: 1,
                            most_detailed_mip: 0,
                        })
                        .build(),
                ),
            )
        }
        .context("Failed to create surface proxy SRV")?;

        Ok(Self {
            view_format: srv.get_desc().format.into(),
            res,
            srv,
            cpu_read,
        })
    }

    pub fn update(&mut self, ctx: &d3d11::DeviceContext, surface: &Surface) {
        if surface.texture.get_desc().resolution() != self.res.get_desc().resolution() {
            *self = Self::new(&ctx.get_device(), surface, self.view_format, self.cpu_read)
                .expect("Failed to resize surface proxy");
        }

        ctx.copy_resource(&surface.texture, &self.res);
    }
}

// Surface helpers
impl Renderer {
    pub fn clear_surface(
        &self,
        cmd: &mut CommandList,
        surface: SurfaceHandle,
        color: impl Into<[f32; 4]>,
    ) {
        self.surfaces().get(surface).clear_color(cmd, color.into());
    }

    pub fn clear_surface_depth(
        &self,
        cmd: &mut CommandList,
        surface: SurfaceHandle,
        depth: f32,
        stencil: u8,
    ) {
        self.surfaces()
            .get(surface)
            .clear_depth(cmd, depth, stencil);
    }

    pub fn bind_surfaces(
        &self,
        cmd: &mut CommandList,
        color: &[SurfaceHandle],
        depth: Option<SurfaceHandle>,
    ) {
        if !color.is_empty() {
            let viewports: SmallVec<[d3d11::Viewport; 4]> = color
                .iter()
                .map(|s| self.surfaces().get(*s).viewport())
                .collect();
            cmd.rasterizer_set_viewports(&viewports);
        } else if let Some(depth) = depth {
            let viewport = self.surfaces().get(depth).viewport();
            cmd.rasterizer_set_viewports(&[viewport]);
        }

        let surfaces = self.surfaces();
        let mut rtvs = SmallVec::<[Option<&d3d11::RenderTargetView>; 8]>::new();
        for s in color {
            rtvs.push(surfaces.get(*s).rtv.as_ref());
        }
        // let rtvs = color
        //     .iter()
        //     .map(|s| self.surfaces().get(*s).rtv)
        //     .collect::<Vec<_>>();
        let dsv = depth.and_then(|s| self.surfaces().get(s).dsv.clone());

        cmd.output_merger_set_render_targets(&rtvs, dsv.as_ref());
    }
}
