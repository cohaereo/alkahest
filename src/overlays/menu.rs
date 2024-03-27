use egui::{vec2, Color32, RichText, Vec2};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use glam::Vec3;

use super::gui::{HiddenWindows, Overlay};
use crate::{
    camera::FpsCamera,
    config,
    ecs::{
        components::{Beacon, Mutable, Ruler, Sphere},
        resources::SelectedEntity,
        tags::{EntityTag, Tags},
        transform::{Transform, TransformFlags},
    },
    icons::{
        ICON_ALPHA_A_BOX_OUTLINE, ICON_ALPHA_D_BOX_OUTLINE, ICON_ALPHA_E_BOX_OUTLINE,
        ICON_ALPHA_F_BOX_OUTLINE, ICON_ALPHA_G_BOX_OUTLINE, ICON_ALPHA_H_BOX_OUTLINE,
        ICON_ALPHA_I_BOX_OUTLINE, ICON_ALPHA_Q_BOX_OUTLINE, ICON_ALPHA_S_BOX_OUTLINE,
        ICON_ALPHA_W_BOX_OUTLINE, ICON_APPLE_KEYBOARD_SHIFT, ICON_ARROW_ALL,
        ICON_ARROW_DOWN_BOLD_BOX_OUTLINE, ICON_ARROW_UP_BOLD_BOX_OUTLINE, ICON_KEYBOARD_SPACE,
        ICON_MOUSE_LEFT_CLICK_OUTLINE, ICON_MOUSE_RIGHT_CLICK_OUTLINE, ICON_RULER_SQUARE,
        ICON_SIGN_POLE, ICON_SPHERE,
    },
    map::MapList,
    updater::UpdateChannel,
    util::consts::{self, CHANGELOG_MD},
    RendererShared,
};

#[derive(Default)]
pub struct MenuBar {
    controls_open: bool,
    changelog_open: bool,
    about_open: bool,
    markdown_cache: CommonMarkCache,
}

macro_rules! control_section_title {
    ($ui:expr, $title:expr) => {{
        $ui.separator();
        $ui.label(RichText::new($title).size(20.0).strong());

        $ui.end_row();
    }};
}

macro_rules! control_description {
    ($ui:expr, $control:expr, $description:expr) => {{
        $ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            ui.label(RichText::new($control).size(24.0).strong());
        });

        $ui.label($description);
        $ui.end_row();
    }};
}

impl Overlay for MenuBar {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &winit::window::Window,
        resources: &mut crate::resources::Resources,
        _gui: &mut super::gui::GuiContext<'_>,
    ) -> bool {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Utility", |ui| {
                    if ui.button(format!("{} Ruler", ICON_RULER_SQUARE)).clicked() {
                        let mut maps = resources.get_mut::<MapList>().unwrap();
                        let renderer = resources.get::<RendererShared>().unwrap();
                        let camera = resources.get::<FpsCamera>().unwrap();
                        let (_, pos) = renderer
                            .read()
                            .gbuffer
                            .depth_buffer_distance_pos_center(&camera);

                        if let Some(map) = maps.current_map_mut() {
                            let position_base = camera.position + camera.front * 15.0;
                            let e = map.scene.spawn((
                                if pos.is_finite() {
                                    Ruler {
                                        start: camera.position,
                                        end: pos,
                                        ..Default::default()
                                    }
                                } else {
                                    Ruler {
                                        start: position_base - camera.right * 10.0,
                                        end: position_base + camera.right * 10.0,
                                        ..Default::default()
                                    }
                                },
                                Tags::from_iter([EntityTag::Utility]),
                                Mutable,
                            ));

                            if let Some(mut se) = resources.get_mut::<SelectedEntity>() {
                                se.0 = Some(e);
                            }

                            ui.close_menu();
                        }
                    }
                    if ui.button(format!("{} Sphere", ICON_SPHERE)).clicked() {
                        let mut maps = resources.get_mut::<MapList>().unwrap();
                        let renderer = resources.get::<RendererShared>().unwrap();
                        let camera = resources.get::<FpsCamera>().unwrap();
                        let (distance, pos) = renderer
                            .read()
                            .gbuffer
                            .depth_buffer_distance_pos_center(&camera);
                        if let Some(map) = maps.current_map_mut() {
                            let camera = resources.get::<FpsCamera>().unwrap();
                            let position_base = camera.position + camera.front * 24.0;
                            let e = map.scene.spawn((
                                Transform {
                                    translation: if distance > 24.0 { position_base } else { pos },
                                    scale: Vec3::splat(9.0),
                                    flags: TransformFlags::IGNORE_ROTATION
                                        | TransformFlags::SCALE_IS_RADIUS,
                                    ..Default::default()
                                },
                                Sphere::default(),
                                Tags::from_iter([EntityTag::Utility]),
                                Mutable,
                            ));

                            if let Some(mut se) = resources.get_mut::<SelectedEntity>() {
                                se.0 = Some(e);
                            }

                            ui.close_menu();
                        }
                    }
                    if ui.button(format!("{} Beacon", ICON_SIGN_POLE)).clicked() {
                        let mut maps: std::cell::RefMut<'_, MapList> =
                            resources.get_mut::<MapList>().unwrap();
                        let renderer = resources.get::<RendererShared>().unwrap();
                        let camera = resources.get::<FpsCamera>().unwrap();
                        let (distance, pos) = renderer
                            .read()
                            .gbuffer
                            .depth_buffer_distance_pos_center(&camera);

                        if let Some(map) = maps.current_map_mut() {
                            let camera = resources.get::<FpsCamera>().unwrap();
                            let e = map.scene.spawn((
                                Transform {
                                    translation: if distance > 24.0 {
                                        camera.position
                                    } else {
                                        pos
                                    },
                                    flags: TransformFlags::IGNORE_ROTATION
                                        | TransformFlags::IGNORE_SCALE,
                                    ..Default::default()
                                },
                                Beacon {
                                    ..Default::default()
                                },
                                Tags::from_iter([EntityTag::Utility]),
                                Mutable,
                            ));

                            if let Some(mut se) = resources.get_mut::<SelectedEntity>() {
                                se.0 = Some(e);
                            }

                            ui.close_menu();
                        }
                    }
                });

                ui.menu_button("View", |ui| {
                    let mut windows = resources.get_mut::<HiddenWindows>().unwrap();
                    windows.texture_dumper ^= ui
                        .selectable_label(windows.texture_dumper, "Texture Dumper")
                        .clicked();

                    windows.tag_dumper ^= ui
                        .selectable_label(windows.tag_dumper, "Tag Dumper")
                        .clicked();
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("Controls").clicked() {
                        self.controls_open = true;
                        ui.close_menu()
                    }
                    ui.separator();
                    let update_channel = config::with(|c| c.update_channel);
                    if ui
                        .add_enabled(
                            update_channel.is_some()
                                && update_channel != Some(UpdateChannel::Disabled),
                            egui::Button::new("Check for updates"),
                        )
                        .clicked()
                    {
                        if let Some(update_channel) = update_channel {
                            resources
                                .get_mut::<crate::updater::UpdateCheck>()
                                .unwrap()
                                .start(update_channel);
                        }
                        ui.close_menu();
                    }

                    if ui.button("Change update channel").clicked() {
                        config::with_mut(|c| c.update_channel = None);
                        ui.close_menu();
                    }

                    if let Some(update_channel) = update_channel {
                        ui.label(format!(
                            "Updates: {} {:?}",
                            update_channel.icon(),
                            update_channel
                        ));
                    }

                    ui.separator();

                    if ui
                        .button("Change package directory")
                        .on_hover_text("Will restart Alkahest")
                        .clicked()
                    {
                        config::with_mut(|c| c.packages_directory = None);
                        config::persist();

                        // Spawn the new process
                        std::process::Command::new(std::env::current_exe().unwrap())
                            .args(std::env::args().skip(1))
                            .spawn()
                            .expect("Failed to spawn the new alkahest process");

                        std::process::exit(0);
                    }

                    if ui.button("Changelog").clicked() {
                        self.changelog_open = true;
                        ui.close_menu();
                    }
                    if ui.button("About").clicked() {
                        self.about_open = true;
                        ui.close_menu();
                    }
                });
            });
        });

        self.change_log(ctx);
        self.about(ctx);
        self.controls(ctx);

        true
    }
}

impl MenuBar {
    pub fn change_log(&mut self, ctx: &egui::Context) {
        egui::Window::new("Changelog")
            .open(&mut self.changelog_open)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    CommonMarkViewer::new("changelog_md").show(
                        ui,
                        &mut self.markdown_cache,
                        CHANGELOG_MD,
                    );
                })
            });
    }
    pub fn about(&mut self, ctx: &egui::Context) {
        egui::Window::new("About")
            .open(&mut self.about_open)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .inner_margin(Vec2::splat(16.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.allocate_ui(vec2(128.0, 128.0), |ui| {
                                ui.add(egui::Image::new(egui::include_image!(
                                    "../../assets/icons/alkahest.png"
                                )));
                            });
                            ui.add_space(16.0);
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.heading(
                                        RichText::new("Alkahest").strong().color(Color32::WHITE),
                                    );
                                    ui.heading(format!("- v{}", consts::VERSION));
                                });
                                ui.separator();
                                ui.label(format!("Revision {}", consts::GIT_HASH));
                                ui.label(format!("Built on {}", consts::BUILD_DATE));
                                if let Ok(v) = rustc_version::version_meta() {
                                    ui.add_space(8.0);
                                    ui.label(format!("rustc {}+{:?}", v.semver, v.channel));
                                }
                            })
                        });
                    })
            });
    }

    pub fn controls(&mut self, ctx: &egui::Context) {
        egui::Window::new("Controls")
            .open(&mut self.controls_open)
            .auto_sized()
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("controls")
                        .min_row_height(30.0)
                        .min_col_width(200.0)
                        .show(ui, |ui| {
                            control_section_title!(ui, "Movement");

                            control_description!(
                                ui,
                                format!("{}+{}", ICON_MOUSE_LEFT_CLICK_OUTLINE, ICON_ARROW_ALL),
                                "Adjust Camera Direction"
                            );

                            control_description!(
                                ui,
                                format!(
                                    "{}/{}/{}/{}",
                                    ICON_ALPHA_W_BOX_OUTLINE,
                                    ICON_ALPHA_S_BOX_OUTLINE,
                                    ICON_ALPHA_A_BOX_OUTLINE,
                                    ICON_ALPHA_D_BOX_OUTLINE
                                ),
                                "Move Camera Forwards/Backwards/Left/Right"
                            );

                            control_description!(
                                ui,
                                format!(
                                    "{}/{}",
                                    ICON_ALPHA_Q_BOX_OUTLINE, ICON_ALPHA_E_BOX_OUTLINE,
                                ),
                                "Move Camera Down/Up"
                            );

                            control_description!(
                                ui,
                                format!(
                                    "Alt + {}/{}/{}/{}",
                                    ICON_ALPHA_W_BOX_OUTLINE,
                                    ICON_ALPHA_S_BOX_OUTLINE,
                                    ICON_ALPHA_A_BOX_OUTLINE,
                                    ICON_ALPHA_D_BOX_OUTLINE
                                ),
                                "Move Camera in Horizontal Plain"
                            );

                            control_description!(
                                ui,
                                format!(
                                    "Alt + {}/{}",
                                    ICON_ALPHA_Q_BOX_OUTLINE, ICON_ALPHA_E_BOX_OUTLINE,
                                ),
                                "Move Camera Down/Up in Absolute Coordinates"
                            );

                            control_description!(ui, "Ctrl", "Decrease Movement speed");

                            control_description!(
                                ui,
                                format!("{} Shift", ICON_APPLE_KEYBOARD_SHIFT),
                                "Increase Movement speed"
                            );

                            control_description!(
                                ui,
                                ICON_KEYBOARD_SPACE,
                                "Increase Movement speed a lot"
                            );

                            control_description!(
                                ui,
                                format!(
                                    "{} Shift + {}",
                                    ICON_APPLE_KEYBOARD_SHIFT, ICON_KEYBOARD_SPACE
                                ),
                                "We're gonna have to go right to... LUDICROUS SPEED"
                            );

                            control_description!(
                                ui,
                                ICON_ALPHA_G_BOX_OUTLINE,
                                "Move Camera to Position of Gaze"
                            );

                            control_section_title!(ui, "Object Interactions");

                            control_description!(
                                ui,
                                ICON_MOUSE_RIGHT_CLICK_OUTLINE,
                                "Select Object"
                            );

                            control_description!(
                                ui,
                                ICON_ALPHA_F_BOX_OUTLINE,
                                "Focus on Selected Object"
                            );

                            control_description!(
                                ui,
                                ICON_ALPHA_H_BOX_OUTLINE,
                                "Toggle Hide Selected Object"
                            );

                            control_description!(
                                ui,
                                format!("Alt + {}", ICON_ALPHA_H_BOX_OUTLINE),
                                "Unhide All Objects"
                            );

                            control_description!(
                                ui,
                                format!(
                                    "{} Shift + {}",
                                    ICON_APPLE_KEYBOARD_SHIFT, ICON_ALPHA_H_BOX_OUTLINE
                                ),
                                "Hide All Unselected Objects"
                            );

                            control_description!(
                                ui,
                                format!("{} Shift + Delete", ICON_APPLE_KEYBOARD_SHIFT),
                                "Delete Selected Object (if allowed)"
                            );

                            control_description!(
                                ui,
                                ICON_ARROW_DOWN_BOLD_BOX_OUTLINE,
                                "Select 'Next' Object"
                            );

                            control_description!(
                                ui,
                                ICON_ARROW_UP_BOLD_BOX_OUTLINE,
                                "Select 'Previous' Object"
                            );
                            control_section_title!(ui, "Miscellaneous");

                            control_description!(
                                ui,
                                ICON_ALPHA_I_BOX_OUTLINE,
                                "Swap to Previous Map"
                            );
                        });
                });
            });
    }
}
