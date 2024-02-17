use egui::{vec2, Color32, RichText, Vec2};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use glam::Vec3;

use super::gui::Overlay;
use crate::{
    camera::FpsCamera,
    config,
    ecs::{
        components::{Beacon, Mutable, Ruler, Sphere},
        resources::SelectedEntity,
        tags::{EntityTag, Tags},
        transform::{Transform, TransformFlags},
    },
    icons::{ICON_RULER_SQUARE, ICON_SIGN_POLE, ICON_SPHERE},
    map::MapDataList,
    updater::UpdateChannel,
    util::consts::{self, CHANGELOG_MD},
    RendererShared,
};

#[derive(Default)]
pub struct MenuBar {
    changelog_open: bool,
    about_open: bool,
    markdown_cache: CommonMarkCache,
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
                        let mut maps = resources.get_mut::<MapDataList>().unwrap();
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
                        let mut maps = resources.get_mut::<MapDataList>().unwrap();
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
                        let mut maps: std::cell::RefMut<'_, MapDataList> =
                            resources.get_mut::<MapDataList>().unwrap();
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

                ui.menu_button("Help", |ui| {
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
}
