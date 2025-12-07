pub mod container;
pub mod definitions;
mod macros;
use std::{any::TypeId, fmt::Debug};

use alkahest_data::tfx::ExternIndex;
pub use container::*;
pub use definitions::*;
use glam::{Mat4, Vec4};

use crate::{
    asset::{texture::Texture, Handle},
    renderer::surface::SurfaceHandle,
    Renderer,
};

trait Extern {
    // fn get_field<T: Sized + 'static>(&self, offset: usize) -> Option<&T> {
    //     let (ptr, typeid) = self.get_field_ptr(offset)?;
    //     if TypeId::of::<T>() != typeid {
    //         return None;
    //     }

    //     let value = unsafe { &*(ptr as *const T) };
    //     Some(value)
    // }

    fn get_field_ptr(&self, offset: usize) -> Option<(*const (), TypeId)>;
    fn get_field_name(offset: usize) -> Option<&'static str>;
}

pub trait ExternAccessor {
    fn get_value_ptr(&self, index: ExternIndex, offset: usize) -> Option<(*const (), TypeId)>;
}

pub trait ExternAccessorExt {
    fn get_extern_value<U: Sized + Clone + ExternValue + 'static>(
        &self,
        index: ExternIndex,
        offset: usize,
    ) -> Option<U>;
}

impl ExternAccessorExt for &dyn ExternAccessor {
    fn get_extern_value<U: Sized + Clone + ExternValue + 'static>(
        &self,
        index: ExternIndex,
        offset: usize,
    ) -> Option<U> {
        let (ptr, typeid) = self.get_value_ptr(index, offset)?;
        if TypeId::of::<U>() != typeid {
            error!(
                "Extern type mismatch for {index:?}+0x{offset:X}: expected {:?} ({}), found {:?}",
                TypeId::of::<U>(),
                std::any::type_name::<U>(),
                typeid
            );
            return None;
        }
        let value = unsafe { &*(ptr as *const U) };
        Some(value.clone())
    }
}

#[derive(Default, Clone)]
pub enum TextureView {
    #[default]
    None,
    Surface(SurfaceHandle),
    Resource(Handle<Texture>),
    Raw(d3d11::ShaderResourceView),
}

impl TextureView {
    pub fn get_srv<F>(&self, f: F)
    where
        F: FnOnce(&d3d11::ShaderResourceView),
    {
        match self {
            TextureView::Surface(surface) => {
                if let Some(srv) = Renderer::instance().surfaces().get(*surface).srv.as_ref() {
                    f(srv)
                }
            }
            TextureView::Resource(texture) => {
                if let Some(t) = texture.get() {
                    f(&t.view)
                }
            }
            TextureView::Raw(srv) => f(srv),
            TextureView::None => {}
        };
    }

    pub fn is_none(&self) -> bool {
        matches!(self, TextureView::None)
    }
}

impl Debug for TextureView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextureView::None => write!(f, "None"),
            TextureView::Surface(surface) => write!(f, "Surface({surface:?})"),
            TextureView::Resource(texture) => write!(f, "Resource({})", texture.hash()),
            TextureView::Raw(_) => write!(f, "Raw"),
        }
    }
}

impl From<SurfaceHandle> for TextureView {
    fn from(surface: SurfaceHandle) -> Self {
        TextureView::Surface(surface)
    }
}

impl From<Handle<Texture>> for TextureView {
    fn from(texture: Handle<Texture>) -> Self {
        TextureView::Resource(texture)
    }
}

impl From<d3d11::ShaderResourceView> for TextureView {
    fn from(srv: d3d11::ShaderResourceView) -> Self {
        TextureView::Raw(srv)
    }
}

impl From<Option<d3d11::ShaderResourceView>> for TextureView {
    fn from(srv: Option<d3d11::ShaderResourceView>) -> Self {
        match srv {
            Some(srv) => TextureView::Raw(srv),
            None => TextureView::None,
        }
    }
}

#[derive(Default, Clone)]
pub enum Uav {
    #[default]
    None,
    Surface(SurfaceHandle),
}

impl Debug for Uav {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Uav::None => write!(f, "None"),
            Uav::Surface(surface) => write!(f, "Surface({surface:?})"),
        }
    }
}

impl Uav {
    pub fn get_uav<F>(&self, f: F)
    where
        F: FnOnce(&d3d11::UnorderedAccessView),
    {
        match self {
            Uav::Surface(surface) => {
                if let Some(uav) = Renderer::instance().surfaces().get(*surface).uav.as_ref() {
                    f(uav)
                }
            }
            Uav::None => {}
        }
    }
}

impl From<SurfaceHandle> for Uav {
    fn from(surface: SurfaceHandle) -> Self {
        Uav::Surface(surface)
    }
}

/// Marker trait for types that can be used as extern values in the TFX system.
pub trait ExternValue {}

impl ExternValue for f32 {}
impl ExternValue for Vec4 {}
impl ExternValue for Mat4 {}
impl ExternValue for TextureView {}
impl ExternValue for Uav {}

pub trait ExternDefault {
    fn extern_default() -> Self;
}

impl ExternDefault for TextureView {
    fn extern_default() -> Self {
        TextureView::None
    }
}

impl ExternDefault for Uav {
    fn extern_default() -> Self {
        Uav::None
    }
}

impl ExternDefault for Vec4 {
    fn extern_default() -> Self {
        Vec4::ONE
    }
}

impl ExternDefault for Mat4 {
    fn extern_default() -> Self {
        Mat4::IDENTITY
    }
}

impl ExternDefault for f32 {
    fn extern_default() -> Self {
        1.0
    }
}

impl ExternDefault for u32 {
    fn extern_default() -> Self {
        0
    }
}
