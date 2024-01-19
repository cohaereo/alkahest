use glam::Vec3;

use crate::{
    camera::FpsCamera,
    ecs::{
        components::{Beacon, Mutable, Ruler, Sphere},
        resources::SelectedEntity,
        tags::{EntityTag, Tags},
        transform::{Transform, TransformFlags},
    },
    icons::{ICON_RULER_SQUARE, ICON_SIGN_POLE, ICON_SPHERE},
    map::MapDataList,
    RendererShared,
};

use super::gui::Overlay;

pub struct MenuBar;

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
                            let camera = resources.get::<FpsCamera>().unwrap();
                            let e = map.scene.spawn((
                                Ruler {
                                    start: pos,
                                    end: camera.position,
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
            });
        });

        true
    }
}
