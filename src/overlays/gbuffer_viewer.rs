use glam::{Mat4, Vec4};
use imgui::{Condition, TreeNodeFlags, WindowFlags};
use std::{fmt::Display, fmt::Formatter};
use winit::window::Window;

use crate::resources::Resources;

use super::gui::OverlayProvider;

pub struct GBufferInfoOverlay {
    pub composition_mode: usize,
    pub map_index: usize,
    pub maps: Vec<(u32, String)>,

    pub renderlayer_statics: bool,
    pub renderlayer_terrain: bool,
    pub renderlayer_entities: bool,
}

impl OverlayProvider for GBufferInfoOverlay {
    fn create_overlay(&mut self, ui: &mut imgui::Ui, _window: &Window, _resources: &mut Resources) {
        ui.window("Options")
            .flags(WindowFlags::NO_TITLE_BAR)
            .size([178.0, 72.0], Condition::FirstUseEver)
            .build(|| {
                ui.combo(" ", &mut self.composition_mode, &COMPOSITOR_MODES, |v| {
                    format!("{v}").into()
                });
                ui.combo("Map", &mut self.map_index, &self.maps, |(i, map_name)| {
                    format!("{map_name} ({:08X})", i.to_be()).into()
                });
                ui.separator();
                if ui.collapsing_header("Render Layers", TreeNodeFlags::empty()) {
                    ui.indent();
                    ui.checkbox("Statics", &mut self.renderlayer_statics);
                    ui.checkbox("Terrain", &mut self.renderlayer_terrain);
                    ui.checkbox("Entities", &mut self.renderlayer_entities);
                    ui.unindent();
                }
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
    // Matcap = 11,
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
    // CompositorMode::Matcap,
];

impl Display for CompositorMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            CompositorMode::Combined => "Combined",
            CompositorMode::Albedo => "Albedo (RT0)",
            CompositorMode::Normal => "Normal (RT1)",
            CompositorMode::PbrStack => "PBR Stack (RT2)",
            CompositorMode::SmoothnessFuzz => "Smoothness/Fuzz",
            CompositorMode::Metalicness => "Metalicness",
            CompositorMode::TextureAO => "Texture AO",
            CompositorMode::Emission => "Emission",
            CompositorMode::Transmission => "Transmission",
            CompositorMode::VertexAO => "Vertex AO",
            CompositorMode::Iridescence => "Iridescence",
            // CompositorMode::Matcap => "Matcap",
        };

        f.write_str(name)
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
