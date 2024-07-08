use crate::gui::activity_select::get_activity_hash;
use crate::gui::menu::MenuBar;
use crate::maplist::MapList;
use alkahest_renderer::camera::Camera;
use alkahest_renderer::ecs::common::{Global, Icon, Label, Mutable};
use alkahest_renderer::ecs::resources::SelectedEntity;
use alkahest_renderer::ecs::tags::{EntityTag, Tags};
use alkahest_renderer::ecs::transform::{Transform, TransformFlags};
use alkahest_renderer::ecs::utility::{Beacon, Route, RouteNode, Ruler, Sphere, Utility};
use alkahest_renderer::ecs::SceneInfo;
use alkahest_renderer::icons::{
    ICON_MAP_MARKER_PATH, ICON_POKEBALL, ICON_RULER_SQUARE, ICON_SIGN_POLE, ICON_SPHERE,
};
use alkahest_renderer::renderer::RendererShared;
use alkahest_renderer::resources::Resources;
use alkahest_renderer::shader::shader_ball::ShaderBallComponent;
use egui::Ui;
use glam::Vec3;

impl MenuBar {
    pub(super) fn utility_menu(&self, ui: &mut Ui, resources: &Resources) {
        if ui.button(format!("{} Ruler", ICON_RULER_SQUARE)).clicked() {
            let mut maps = resources.get_mut::<MapList>();
            let renderer = resources.get::<RendererShared>();
            let camera = resources.get::<Camera>();
            let (_, pos) = renderer
                .data
                .lock()
                .gbuffers
                .depth_buffer_distance_pos_center(&camera);

            if let Some(map) = maps.current_map_mut() {
                let position_base = camera.position() + camera.forward() * 15.0;
                let e = map.scene.spawn((
                    if pos.is_finite() {
                        Ruler {
                            start: camera.position(),
                            end: pos,
                            ..Default::default()
                        }
                    } else {
                        Ruler {
                            start: position_base - camera.right() * 10.0,
                            end: position_base + camera.right() * 10.0,
                            ..Default::default()
                        }
                    },
                    Ruler::icon(),
                    Ruler::default_label(),
                    Tags::from_iter([EntityTag::Utility]),
                    Mutable,
                ));

                resources.get_mut::<SelectedEntity>().select(e);

                ui.close_menu();
            }
        }
        if ui.button(format!("{} Sphere", ICON_SPHERE)).clicked() {
            let mut maps = resources.get_mut::<MapList>();
            let renderer = resources.get::<RendererShared>();
            let camera = resources.get::<Camera>();
            let (distance, pos) = renderer
                .data
                .lock()
                .gbuffers
                .depth_buffer_distance_pos_center(&camera);
            if let Some(map) = maps.current_map_mut() {
                let camera = resources.get::<Camera>();
                let position_base = camera.position() + camera.forward() * 24.0;
                let e = map.scene.spawn((
                    Transform {
                        translation: if distance > 24.0 { position_base } else { pos },
                        scale: Vec3::splat(9.0),
                        flags: TransformFlags::IGNORE_ROTATION | TransformFlags::SCALE_IS_RADIUS,
                        ..Default::default()
                    },
                    Sphere::default(),
                    Sphere::icon(),
                    Sphere::default_label(),
                    Tags::from_iter([EntityTag::Utility]),
                    Mutable,
                ));

                resources.get_mut::<SelectedEntity>().select(e);

                ui.close_menu();
            }
        }
        if ui.button(format!("{} Beacon", ICON_SIGN_POLE)).clicked() {
            let mut maps: std::cell::RefMut<'_, MapList> = resources.get_mut::<MapList>();
            let renderer = resources.get::<RendererShared>();
            let camera = resources.get::<Camera>();
            let (distance, pos) = renderer
                .data
                .lock()
                .gbuffers
                .depth_buffer_distance_pos_center(&camera);

            if let Some(map) = maps.current_map_mut() {
                let camera = resources.get::<Camera>();
                let e = map.scene.spawn((
                    Transform {
                        translation: if distance > 24.0 {
                            camera.position()
                        } else {
                            pos
                        },
                        flags: TransformFlags::IGNORE_ROTATION | TransformFlags::IGNORE_SCALE,
                        ..Default::default()
                    },
                    Beacon::default(),
                    Beacon::icon(),
                    Beacon::default_label(),
                    Tags::from_iter([EntityTag::Utility]),
                    Mutable,
                ));

                resources.get_mut::<SelectedEntity>().select(e);

                ui.close_menu();
            }
        }
        if ui
            .button(format!("{} Route", ICON_MAP_MARKER_PATH))
            .clicked()
        {
            let mut maps = resources.get_mut::<MapList>();
            let camera = resources.get::<Camera>();

            if let Some(map) = maps.current_map_mut() {
                let e = map.scene.spawn((
                    Route {
                        path: vec![RouteNode {
                            pos: camera.position(),
                            map_hash: map.scene.get_map_hash(),
                            is_teleport: false,
                            label: None,
                        }],
                        activity_hash: map.scene.get_activity_hash(),
                        ..Default::default()
                    },
                    Tags::from_iter([EntityTag::Utility, EntityTag::Global]),
                    Route::icon(),
                    Route::default_label(),
                    Mutable,
                    Global,
                ));

                resources.get_mut::<SelectedEntity>().select(e);

                ui.close_menu();
            }
        }

        ui.separator();

        if ui
            .button(format!("{} Material Ball", ICON_POKEBALL))
            .clicked()
        {
            let mut maps: std::cell::RefMut<'_, MapList> = resources.get_mut::<MapList>();
            let renderer = resources.get::<RendererShared>();

            if let Some(map) = maps.current_map_mut() {
                let camera = resources.get::<Camera>();
                let e = map.scene.spawn((
                    Icon::Unicode(ICON_POKEBALL),
                    Label::from("Material Ball"),
                    Transform::from_translation(camera.position()),
                    ShaderBallComponent::new(&renderer).unwrap(),
                    Tags::from_iter([EntityTag::Utility]),
                    Mutable,
                ));

                resources.get_mut::<SelectedEntity>().select(e);

                ui.close_menu();
            }
        }
    }
}
