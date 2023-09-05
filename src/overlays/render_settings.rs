use glam::{Mat4, Vec4};
use imgui::{Condition, TreeNodeFlags, WindowFlags};
use std::{fmt::Display, fmt::Formatter};
use winit::window::Window;

use crate::{map::MapDataList, resources::Resources};

use super::gui::OverlayProvider;

pub struct RenderSettingsOverlay {
    pub composition_mode: usize,

    pub renderlayer_statics: bool,
    pub renderlayer_statics_transparent: bool,
    pub renderlayer_terrain: bool,
    pub renderlayer_entities: bool,

    pub render_lights: bool,
    pub alpha_blending: bool,
    pub blend_override: usize,
}

impl OverlayProvider for RenderSettingsOverlay {
    fn create_overlay(&mut self, ui: &mut imgui::Ui, _window: &Window, resources: &mut Resources) {
        ui.window("Options")
            .flags(WindowFlags::NO_TITLE_BAR)
            .size([178.0, 72.0], Condition::FirstUseEver)
            .build(|| {
                ui.checkbox("Render lights", &mut self.render_lights);
                ui.checkbox("Enable color blending", &mut self.alpha_blending);
                if self.alpha_blending {
                    ui.combo_simple_string(
                        "Blend Override",
                        &mut self.blend_override,
                        &["Default", "Blend", "Additive"],
                    );
                }
                if ui.collapsing_header("Render Layers", TreeNodeFlags::DEFAULT_OPEN) {
                    ui.indent();
                    ui.checkbox("Statics", &mut self.renderlayer_statics);
                    ui.checkbox(
                        "Statics (overlay/transparent)",
                        &mut self.renderlayer_statics_transparent,
                    );
                    ui.checkbox("Terrain", &mut self.renderlayer_terrain);
                    ui.checkbox("Entities", &mut self.renderlayer_entities);
                    ui.unindent();
                }
            });

        ui.window("Selectors")
            .flags(
                WindowFlags::NO_BACKGROUND
                    | WindowFlags::NO_TITLE_BAR
                    | WindowFlags::NO_DECORATION
                    | WindowFlags::NO_RESIZE
                    | WindowFlags::NO_MOVE,
            )
            .save_settings(false)
            // .size([416.0, 20.0], Condition::Always)
            .position([0.0, 0.0], Condition::Always)
            .build(|| {
                let width = ui.push_item_width(128.0);
                ui.combo("Pass", &mut self.composition_mode, COMPOSITOR_MODES, |v| {
                    v.to_string().into()
                });
                width.end();
                ui.same_line();
                let mut maps = resources.get_mut::<MapDataList>().unwrap();
                let mut current_map = maps.current_map;
                let width = ui.push_item_width(272.0);
                ui.combo("Map", &mut current_map, &maps.maps, |m| {
                    format!("{} ({})", m.name, m.hash).into()
                });
                width.end();
                maps.current_map = current_map;
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
    Cubemap = 11,
}

pub const COMPOSITOR_MODES: &[CompositorMode] = &[
    CompositorMode::Combined,       // 0
    CompositorMode::Albedo,         // 1
    CompositorMode::Normal,         // 2
    CompositorMode::PbrStack,       // 3
    CompositorMode::SmoothnessFuzz, // 4
    CompositorMode::Metalicness,    // 5
    CompositorMode::TextureAO,      // 6
    CompositorMode::Emission,       // 7
    CompositorMode::Transmission,   // 8
    CompositorMode::VertexAO,       // 9
    CompositorMode::Iridescence,    // 10
    CompositorMode::Cubemap,        // 11
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
            CompositorMode::Cubemap => "Cubemap",
        };

        f.write_str(name)
    }
}

#[repr(C)]
pub struct CompositorOptions {
    pub proj_view_matrix_inv: Mat4,
    pub proj_matrix: Mat4,
    pub view_matrix: Mat4,
    pub camera_pos: Vec4,
    pub camera_dir: Vec4,
    pub time: f32,
    pub mode: u32,
    pub light_count: u32,
}
