use const_format::concatcp;
use glam::{Mat4, Vec3, Vec4};
use itertools::Itertools;
use nohash_hasher::{IntMap, IntSet};
use std::{fmt::Display, fmt::Formatter, mem::transmute, time::Instant};
use winit::window::Window;

use crate::{
    ecs::components::ActivityGroup,
    map::MapDataList,
    render::{
        overrides::{EnabledShaderOverrides, ScopeOverrides},
        renderer::ShadowMapsResource,
    },
    resources::Resources,
};

use super::gui::Overlay;

pub struct RenderSettingsOverlay {
    pub renderlayer_statics: bool,
    pub renderlayer_statics_transparent: bool,
    pub renderlayer_terrain: bool,
    pub renderlayer_entities: bool,
    pub renderlayer_background: bool,

    pub shadow_res_index: usize,
    pub animate_light: bool,
    pub light_dir_degrees: Vec3,
    pub last_frame: Instant,
}

impl Overlay for RenderSettingsOverlay {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &Window,
        resources: &mut Resources,
        _gui: super::gui::GuiContext<'_>,
    ) -> bool {
        let delta_time = self.last_frame.elapsed().as_secs_f32();
        self.last_frame = Instant::now();

        let mut render_settings = resources.get_mut::<RenderSettings>().unwrap();
        egui::Window::new("Options").show(ctx, |ui| {
            ui.checkbox(&mut render_settings.draw_lights, "Render lights");
            ui.indent("render settings specular option indent", |ui| {
                ui.add_enabled(
                    render_settings.draw_lights,
                    egui::Checkbox::new(&mut render_settings.use_specular_map, "Use Specular Maps"),
                );
            });

            ui.checkbox(&mut render_settings.fxaa, "Anti-aliasing");

            ui.checkbox(
                &mut render_settings.evaluate_bytecode,
                "Evaluate TFX bytecode (WIP)",
            );
            ui.checkbox(&mut render_settings.alpha_blending, "Enable color blending");
            if render_settings.alpha_blending {
                egui::ComboBox::from_label("Blend Override").show_index(
                    ui,
                    &mut render_settings.blend_override,
                    4,
                    |i| {
                        [
                            "Default",
                            concatcp!(crate::icons::ICON_VECTOR_DIFFERENCE, " Blend"),
                            concatcp!(crate::icons::ICON_PLUS, " Additive"),
                            concatcp!(crate::icons::ICON_CLOSE, " Discard"),
                        ][i]
                    },
                );
            }

            let mut c = render_settings.clear_color.to_array();
            ui.horizontal(|ui| {
                ui.color_edit_button_rgb(unsafe { transmute(&mut c) });
                ui.label("Clear color");
            });
            c[3] = 1.0;
            render_settings.clear_color = Vec4::from_array(c);

            {
                const SHADOW_RESOLUTIONS: &[usize] = &[2048, 4096, 8192, 16384];
                let response = egui::ComboBox::from_label("Shadow Resolution").show_index(
                    ui,
                    &mut self.shadow_res_index,
                    SHADOW_RESOLUTIONS.len(),
                    |i| {
                        if SHADOW_RESOLUTIONS[i] > 8192 {
                            format!("{} (may crash)", SHADOW_RESOLUTIONS[i].to_string())
                        } else {
                            SHADOW_RESOLUTIONS[i].to_string()
                        }
                    },
                );

                if response.changed() {
                    let mut csb = resources.get_mut::<ShadowMapsResource>().unwrap();
                    csb.resize(SHADOW_RESOLUTIONS[self.shadow_res_index]);
                }
            }

            ui.horizontal(|ui| {
                ui.strong("Directional Light");
                ui.checkbox(&mut self.animate_light, "Animate");
            });

            if self.animate_light {
                self.light_dir_degrees.z += delta_time * 15.0;
                self.light_dir_degrees.z %= 360.0;
            }

            ui.add(
                egui::Slider::new(&mut self.light_dir_degrees.x, 0.0..=2.0)
                    .text("Angle")
                    .fixed_decimals(1),
            );
            ui.add_enabled_ui(!self.animate_light, |ui| {
                ui.add(
                    egui::Slider::new(&mut self.light_dir_degrees.z, 0.0..=360.0)
                        .text("Rotation")
                        .fixed_decimals(1),
                );
            });

            render_settings.light_dir = Vec3::new(
                self.light_dir_degrees.z.to_radians().sin(),
                self.light_dir_degrees.z.to_radians().cos(),
                self.light_dir_degrees.x,
            );

            ui.separator();

            ui.collapsing("Render Layers", |ui| {
                ui.checkbox(&mut self.renderlayer_statics, "Statics");
                ui.checkbox(
                    &mut self.renderlayer_statics_transparent,
                    "Statics (overlay/transparent)",
                );
                ui.checkbox(&mut self.renderlayer_terrain, "Terrain");
                ui.checkbox(&mut self.renderlayer_entities, "Entities");
                ui.checkbox(&mut self.renderlayer_background, "Background Entities");
            });

            if let Some(mut enabled_overrides) = resources.get_mut::<EnabledShaderOverrides>() {
                ui.collapsing("Shader Overrides", |ui| {
                    ui.checkbox(&mut enabled_overrides.entity_vs, "Entity (VS)");
                    ui.checkbox(&mut enabled_overrides.entity_ps, "Entity (PS)");
                    ui.checkbox(
                        &mut enabled_overrides.terrain_ps,
                        "Terrain texturemap debug (PS)",
                    );
                });
            }

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

                macro_rules! input_float {
                    ($ui:expr, $label:expr, $v:expr) => {
                        $ui.horizontal(|ui| {
                            ui.label($label);
                            ui.add(egui::DragValue::new(&mut $v).speed(0.1));
                        });
                    };
                }

                ui.collapsing("frame", |ui| {
                    input_float!(ui, "exposure_scale", overrides.frame.exposure_scale);
                    input_float!(
                        ui,
                        "exposure_illum_relative_glow",
                        overrides.frame.exposure_illum_relative_glow
                    );
                    input_float!(
                        ui,
                        "exposure_scale_for_shading",
                        overrides.frame.exposure_scale_for_shading
                    );
                    input_float!(
                        ui,
                        "exposure_illum_relative",
                        overrides.frame.exposure_illum_relative
                    );

                    input_float4!(ui, "random_seed_scales", overrides.frame.random_seed_scales);
                    input_float4!(ui, "overrides", overrides.frame.overrides);
                    ui.separator();
                    input_float4!(ui, "unk4", overrides.frame.unk4);
                    input_float4!(ui, "unk5", overrides.frame.unk5);
                    input_float4!(ui, "unk6", overrides.frame.unk6);
                    input_float4!(ui, "unk7", overrides.frame.unk7);
                });

                ui.collapsing("unk2", |ui| {
                    input_float4!(ui, "unk0", overrides.unk2.unk0);
                });

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
                    &mut render_settings.compositor_mode,
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
                    |i| &maps.maps[i].2.name,
                );
                ui.label(format!("Map hash: {}", maps.maps[maps.current_map].0));
                ui.label(format!(
                    "Map hash64: {}",
                    maps.maps[maps.current_map].1.unwrap_or_default()
                ));

                maps.current_map = current_map;

                let groups_in_current_scene: IntSet<u32> = maps
                    .current_map()
                    .unwrap()
                    .2
                    .scene
                    .query::<&ActivityGroup>()
                    .iter()
                    .map(|(_, ag)| ag.0)
                    .collect();

                if !groups_in_current_scene.is_empty() {
                    ui.collapsing("Activity Groups", |ui| {
                        let mut groups = resources.get_mut::<ActivityGroupFilter>().unwrap();
                        // Remove old groups
                        for g in groups.filters.keys().cloned().collect_vec() {
                            if !groups_in_current_scene.contains(&g) {
                                groups.filters.remove(&g);
                            }
                        }

                        // Add new groups
                        for g in &groups_in_current_scene {
                            groups.filters.entry(*g).or_insert(true);
                        }

                        for (id, enabled) in groups.filters.iter_mut() {
                            ui.checkbox(enabled, format!("{id:08X}"));
                        }
                    });
                }
            }
        });

        true
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
    Depth = 13,
    Specular = 14,
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
    CompositorMode::Depth,          // 13
    CompositorMode::Specular,       // 14
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
            CompositorMode::Depth => "Depth",
            CompositorMode::Specular => "Specular",
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
    pub light_dir: Vec4,
    pub specular_scale: f32,
    pub fxaa_enabled: u32,
}

pub struct RenderSettings {
    pub draw_lights: bool,
    pub alpha_blending: bool,
    pub compositor_mode: usize,
    pub blend_override: usize,
    pub evaluate_bytecode: bool,
    pub clear_color: Vec4,
    pub light_dir: Vec3,
    pub use_specular_map: bool,
    pub fxaa: bool,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            compositor_mode: CompositorMode::Combined as usize,
            alpha_blending: true,
            draw_lights: false,
            blend_override: 0,
            evaluate_bytecode: false,
            clear_color: Vec4::ZERO,
            light_dir: Vec3::NEG_Z,
            use_specular_map: true,
            fxaa: true,
        }
    }
}

#[derive(Default)]
pub struct ActivityGroupFilter {
    pub filters: IntMap<u32, bool>,
}
