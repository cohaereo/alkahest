use std::{fmt::Formatter, fmt::Display};
use imgui::{WindowFlags, Condition};
use winit::window::Window;

use super::gui::OverlayProvider;

pub struct GBufferInfoOverlay {
    pub composition_mode: usize
 }

impl OverlayProvider for GBufferInfoOverlay {
    fn create_overlay(&mut self, ui: &mut imgui::Ui, window: &Window) {
        ui.window("Options")
            .flags(WindowFlags::NO_TITLE_BAR | WindowFlags::NO_RESIZE)
            .size([128.0, 36.0], Condition::Always)
            .build(|| {
                ui.combo(" ", &mut self.composition_mode, &COMPOSITOR_MODES, |v| {
                format!("{v}").into()
            });
        });
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum CompositorMode {
    /// Rendered output
    Combined = 0,

    /// RT0
    Albedo = 1,
    /// RT1
    Normal = 2,
    /// RT2
    PbrStack = 3,

    SmoothnessFuzz = 4,
    Metalicness = 5,
    TextureAO = 6,
    Emission = 7,
    Transmission = 8,
    VertexAO = 9,
    Iridescence = 10,
}

pub const COMPOSITOR_MODES: &[CompositorMode] = &[
    CompositorMode::Combined,
    CompositorMode::Albedo,
    CompositorMode::Normal,
    CompositorMode::PbrStack,
    CompositorMode::SmoothnessFuzz,
    CompositorMode::Metalicness,
    CompositorMode::Emission,
    CompositorMode::Transmission,
    CompositorMode::Iridescence,
    CompositorMode::TextureAO,
    CompositorMode::VertexAO,
];

impl Display for CompositorMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CompositorMode::Combined => f.write_str("Combined"),
            CompositorMode::Albedo => f.write_str("Albedo (RT0)"),
            CompositorMode::Normal => f.write_str("Normal (RT1)"),
            CompositorMode::PbrStack => f.write_str("PBR Stack (RT2)"),
            CompositorMode::SmoothnessFuzz => f.write_str("Smoothness/Fuzz"),
            CompositorMode::Metalicness => f.write_str("Metalicness"),
            CompositorMode::TextureAO => f.write_str("Texture AO"),
            CompositorMode::Emission => f.write_str("Emission"),
            CompositorMode::Transmission => f.write_str("Transmission"),
            CompositorMode::VertexAO => f.write_str("Vertex AO"),
            CompositorMode::Iridescence => f.write_str("Iridescence"),
        }
    }
}