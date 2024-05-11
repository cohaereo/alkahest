use std::{cell::RefCell, ops::Deref, time::Instant};

use glam::Vec4;
use lazy_static::lazy_static;

pub type Color = ecolor::Rgba;

pub trait ColorExt {
    fn text_color_for_background(&self) -> Self;
    fn to_vec4(&self) -> Vec4;
    fn is_opaque(&self) -> bool;
    fn invert(&self) -> Self;
    fn keep_bright(&self) -> Self;
}

impl ColorExt for ecolor::Rgba {
    fn text_color_for_background(&self) -> Self {
        let luma = 0.2126 * self.r() + 0.7152 * self.g() + 0.0722 * self.b();

        if luma > 0.5 {
            Self::BLACK
        } else {
            Self::WHITE
        }
    }

    fn to_vec4(&self) -> Vec4 {
        Vec4::from(self.to_array())
    }

    fn is_opaque(&self) -> bool {
        self.a() == 1.0
    }

    fn invert(&self) -> Self {
        Self::from_rgba_premultiplied(1.0 - self.r(), 1.0 - self.g(), 1.0 - self.b(), self.a())
    }

    fn keep_bright(&self) -> Self {
        Self::from_rgba_premultiplied(
            self.r().max(0.5),
            self.g().max(0.5),
            self.b().max(0.5),
            self.a(),
        )
    }
}

impl ColorExt for ecolor::Color32 {
    fn text_color_for_background(&self) -> Self {
        let luma = 0.2126 * self.r() as f32 + 0.7152 * self.g() as f32 + 0.0722 * self.b() as f32;

        if luma > 0.5 {
            Self::BLACK
        } else {
            Self::WHITE
        }
    }

    fn to_vec4(&self) -> Vec4 {
        Color::from(*self).to_vec4()
    }

    fn is_opaque(&self) -> bool {
        self.a() == 255
    }

    fn invert(&self) -> Self {
        Self::from_rgba_premultiplied(
            255 - self.r(),
            255 - self.g(),
            255 - self.b(),
            self.a(),
        )
    }

    fn keep_bright(&self) -> Self {
        Self::from_rgba_premultiplied(
            self.r().max(128),
            self.g().max(128),
            self.b().max(128),
            self.a(),
        )
    }
}

pub struct Hsv(ecolor::Hsva);

lazy_static! {
    static ref START_TIME: Instant = Instant::now();
}

impl Hsv {
    pub fn new(h: f32, s: f32, v: f32) -> Self {
        Hsv(ecolor::Hsva::new(h, s, v, 1.0))
    }

    pub fn rainbow() -> Self {
        let start_time = START_TIME.elapsed().as_secs_f32();
        let h = start_time * 0.30 % 1.0;
        let s = 1.0;
        let v = 1.0;
        Hsv::new(h, s, v)
    }
}

impl Deref for Hsv {
    type Target = ecolor::Hsva;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
