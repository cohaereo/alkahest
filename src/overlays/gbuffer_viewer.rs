use destiny_pkg::TagHash;
use glam::{Mat4, Vec4};
use imgui::{Condition, WindowFlags};
use std::{fmt::Display, fmt::Formatter};
use winit::window::Window;

use super::{gui::OverlayProvider, resource_nametags::ResourcePoint};

pub struct GBufferInfoOverlay {
    pub composition_mode: usize,
    pub map_index: usize,
    pub maps: Vec<(u32, String, Vec<TagHash>, Vec<ResourcePoint>, Vec<TagHash>)>,
}

impl OverlayProvider for GBufferInfoOverlay {
    fn create_overlay(&mut self, ui: &mut imgui::Ui, _window: &Window) {
        ui.window("Options")
            .flags(WindowFlags::NO_TITLE_BAR)
            .size([178.0, 72.0], Condition::FirstUseEver)
            .build(|| {
                ui.combo(" ", &mut self.composition_mode, &COMPOSITOR_MODES, |v| {
                    format!("{v}").into()
                });
                ui.combo(
                    "Map",
                    &mut self.map_index,
                    &self.maps,
                    |(i, map_name, _, _, _)| format!("{map_name} ({i:x})").into(),
                );
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

#[repr(C)]
pub struct CompositorOptions {
    pub proj_view_matrix_inv: Mat4,
    pub camera_pos: Vec4,
    pub camera_dir: Vec4,
    pub mode: u32,
    pub light_count: u32,
}
