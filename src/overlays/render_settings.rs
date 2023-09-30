use glam::{Mat4, Vec4};
use std::{fmt::Display, fmt::Formatter};
use winit::window::Window;

use crate::{map::MapDataList, render::renderer::ScopeOverrides, resources::Resources};

use super::gui::{GuiResources, OverlayProvider};

pub struct RenderSettingsOverlay {
    pub composition_mode: usize,

    pub renderlayer_statics: bool,
    pub renderlayer_statics_transparent: bool,
    pub renderlayer_terrain: bool,
    pub renderlayer_entities: bool,

    pub render_lights: bool,
    pub alpha_blending: bool,
    pub blend_override: usize,
    pub evaluate_bytecode: bool,
}

impl OverlayProvider for RenderSettingsOverlay {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &Window,
        resources: &mut Resources,
        _icons: &GuiResources,
    ) {
        egui::Window::new("Options").show(ctx, |ui| {
            ui.checkbox(&mut self.render_lights, "Render lights");
            ui.checkbox(&mut self.evaluate_bytecode, "Evaluate TFX bytecode (WIP)");
            ui.checkbox(&mut self.alpha_blending, "Enable color blending");
            if self.alpha_blending {
                egui::ComboBox::from_label("Blend Override").show_index(
                    ui,
                    &mut self.blend_override,
                    3,
                    |i| ["Default", "Blend", "Additive"][i],
                );
            }

            ui.collapsing("Render Layers", |ui| {
                ui.checkbox(&mut self.renderlayer_statics, "Statics");
                ui.checkbox(
                    &mut self.renderlayer_statics_transparent,
                    "Statics (overlay/transparent)",
                );
                ui.checkbox(&mut self.renderlayer_terrain, "Terrain");
                ui.checkbox(&mut self.renderlayer_entities, "Entities");
            });

            ui.collapsing("Scope Overrides", |ui| {
                let mut overrides = resources.get_mut::<ScopeOverrides>().unwrap();

                macro_rules! input_float4 {
                    ($ui:expr, $label:expr, $v:expr) => {
                        $ui.horizontal(|ui| {
                            ui.label($label);
                            ui.add(egui::DragValue::new(&mut $v.x).speed(0.1).prefix("x: "));
                            ui.add(egui::DragValue::new(&mut $v.y).speed(0.1).prefix("y: "));
                            ui.add(egui::DragValue::new(&mut $v.z).speed(0.1).prefix("z: "));
                            ui.add(egui::DragValue::new(&mut $v.w).speed(0.1).prefix("w: "));
                        });
                    };
                }

                ui.collapsing("unk3", |ui| {
                    input_float4!(ui, "unk0", overrides.unk3.unk0);
                    input_float4!(ui, "unk1", overrides.unk3.unk1);
                    input_float4!(ui, "unk2", overrides.unk3.unk2);
                    input_float4!(ui, "unk3", overrides.unk3.unk3);
                    input_float4!(ui, "unk4", overrides.unk3.unk4);
                    input_float4!(ui, "unk5", overrides.unk3.unk5);
                    input_float4!(ui, "unk6", overrides.unk3.unk6);
                    input_float4!(ui, "unk7", overrides.unk3.unk7);
                    input_float4!(ui, "unk8", overrides.unk3.unk8);
                    input_float4!(ui, "unk9", overrides.unk3.unk9);
                    input_float4!(ui, "unk10", overrides.unk3.unk10);
                    input_float4!(ui, "unk11", overrides.unk3.unk11);
                    input_float4!(ui, "unk12", overrides.unk3.unk12);
                    input_float4!(ui, "unk13", overrides.unk3.unk13);
                    input_float4!(ui, "unk14", overrides.unk3.unk14);
                    input_float4!(ui, "unk15", overrides.unk3.unk15);
                });

                ui.collapsing("unk8", |ui| {
                    input_float4!(ui, "unk0", overrides.unk8.unk0);
                    input_float4!(ui, "unk1", overrides.unk8.unk1);
                    input_float4!(ui, "unk2", overrides.unk8.unk2);
                    input_float4!(ui, "unk3", overrides.unk8.unk3);
                    input_float4!(ui, "unk4", overrides.unk8.unk4);
                    input_float4!(ui, "unk5", overrides.unk8.unk5);
                    input_float4!(ui, "unk6", overrides.unk8.unk6);
                    input_float4!(ui, "unk7", overrides.unk8.unk7);
                    input_float4!(ui, "unk8", overrides.unk8.unk8);
                    input_float4!(ui, "unk9", overrides.unk8.unk9);
                    input_float4!(ui, "unk10", overrides.unk8.unk10);
                    input_float4!(ui, "unk11", overrides.unk8.unk11);
                    input_float4!(ui, "unk12", overrides.unk8.unk12);
                    input_float4!(ui, "unk13", overrides.unk8.unk13);
                    input_float4!(ui, "unk14", overrides.unk8.unk14);
                    input_float4!(ui, "unk15", overrides.unk8.unk15);
                    input_float4!(ui, "unk16", overrides.unk8.unk16);
                    input_float4!(ui, "unk17", overrides.unk8.unk17);
                    input_float4!(ui, "unk18", overrides.unk8.unk18);
                    input_float4!(ui, "unk19", overrides.unk8.unk19);
                    input_float4!(ui, "unk20", overrides.unk8.unk20);
                    input_float4!(ui, "unk21", overrides.unk8.unk21);
                    input_float4!(ui, "unk22", overrides.unk8.unk22);
                    input_float4!(ui, "unk23", overrides.unk8.unk23);
                    input_float4!(ui, "unk24", overrides.unk8.unk24);
                    input_float4!(ui, "unk25", overrides.unk8.unk25);
                    input_float4!(ui, "unk26", overrides.unk8.unk26);
                    input_float4!(ui, "unk27", overrides.unk8.unk27);
                    input_float4!(ui, "unk28", overrides.unk8.unk28);
                    input_float4!(ui, "unk29", overrides.unk8.unk29);
                    input_float4!(ui, "unk30", overrides.unk8.unk30);
                    input_float4!(ui, "unk31", overrides.unk8.unk31);
                    input_float4!(ui, "unk32", overrides.unk8.unk32);
                    input_float4!(ui, "unk33", overrides.unk8.unk33);
                    input_float4!(ui, "unk34", overrides.unk8.unk34);
                    input_float4!(ui, "unk35", overrides.unk8.unk35);
                    input_float4!(ui, "unk36", overrides.unk8.unk36);
                })
            });
        });

        egui::Window::new("Selectors").show(ctx, |ui| {
            egui::ComboBox::from_label("Render Pass")
                .width(192.0)
                .show_index(
                    ui,
                    &mut self.composition_mode,
                    COMPOSITOR_MODES.len(),
                    |i| COMPOSITOR_MODES[i].to_string(),
                );

            let mut maps = resources.get_mut::<MapDataList>().unwrap();
            if !maps.maps.is_empty() {
                let mut current_map = maps.current_map;
                egui::ComboBox::from_label("Map").width(192.0).show_index(
                    ui,
                    &mut current_map,
                    maps.maps.len(),
                    |i| &maps.maps[i].1.name,
                );
                ui.label(format!("Map hash: {}", maps.maps[maps.current_map].0));

                maps.current_map = current_map;
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
    Cubemap = 11,
    Matcap = 12,
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
    CompositorMode::Matcap,         // 12
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
            CompositorMode::Matcap => "Matcap",
        };

        f.write_str(name)
    }
}

#[repr(C)]
pub struct CompositorOptions {
    pub proj_view_matrix_inv: Mat4,
    pub proj_view_matrix: Mat4,
    pub proj_matrix: Mat4,
    pub view_matrix: Mat4,
    pub camera_pos: Vec4,
    pub camera_dir: Vec4,
    pub time: f32,
    pub mode: u32,
    pub light_count: u32,
}
