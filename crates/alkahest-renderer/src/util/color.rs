use std::{cell::RefCell, ops::Deref, time::Instant};

use glam::Vec4;
use lazy_static::lazy_static;

pub type Color = ecolor::Rgba;

pub trait ColorExt {
    fn text_color_for_background(&self) -> Self;
    fn to_vec4(&self) -> Vec4;
    fn is_opaque(&self) -> bool;
}

impl ColorExt for Color {
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
