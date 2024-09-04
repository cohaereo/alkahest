use alkahest_renderer::{
    camera::{tween::Tween, Camera},
    ecs::{
        hierarchy::{Children, Parent},
        resources::SelectedEntity,
        transform::Transform,
        utility::{Beacon, Route, RouteNode, RouteNodeHolder, Ruler, Sphere, Utility},
        Scene, SceneInfo,
    },
    icons::{
        ICON_ALERT, ICON_ALPHA_A_BOX, ICON_ALPHA_B_BOX, ICON_ARROW_LEFT, ICON_ARROW_RIGHT,
        ICON_CAMERA, ICON_CAMERA_CONTROL, ICON_CLIPBOARD, ICON_EYE_ARROW_RIGHT_OUTLINE,
        ICON_EYE_OFF_OUTLINE, ICON_MAP_MARKER, ICON_MAP_MARKER_PATH, ICON_MAP_MARKER_PLUS,
    },
    renderer::RendererShared,
    util::{black_magic::EntityRefDarkMagic, text::prettify_distance},
};
use bevy_ecs::{prelude::EntityRef, system::Commands};
use egui::{
    color_picker::{color_edit_button_rgba, Alpha},
    Button,
};

use crate::{
    gui::{
        hotkeys::{SHORTCUT_ADD_ROUTE_NODE_NEXT, SHORTCUT_ADD_ROUTE_NODE_PREV},
        inspector::ComponentPanel,
    },
    input_float3,
    resources::AppResources,
    util::action::{ActionList, FollowAction},
};

impl ComponentPanel for Ruler {
    fn inspector_name() -> &'static str {
        "Ruler"
    }

    fn inspector_icon() -> char {
        Ruler::icon().char()
    }

    fn show_inspector_ui(
        &mut self,
        _: &mut Scene,
        _: &mut Commands<'_, '_>,
        _: EntityRef<'_>,
        ui: &mut egui::Ui,
        resources: &AppResources,
    ) {
        let camera = resources.get::<Camera>();
        egui::Grid::new("transform_input_grid")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                let (d, pos) = resources
                    .get::<RendererShared>()
                    .data
                    .lock()
                    .gbuffers
                    .depth_buffer_distance_pos_center(&camera);
                input_float3!(ui, format!("{ICON_ALPHA_A_BOX} Start"), &mut self.start);
                ui.horizontal(|ui| {
                    if ui
                        .button(ICON_CAMERA_CONTROL.to_string())
                        .on_hover_text("Set position to camera")
                        .clicked()
                    {
                        self.start = camera.position();
                    }

                    if ui
                        .add_enabled(
                            d.is_finite(),
                            Button::new(if d.is_finite() {
                                ICON_EYE_ARROW_RIGHT_OUTLINE.to_string()
                            } else {
                                ICON_EYE_OFF_OUTLINE.to_string()
                            }),
                        )
                        .on_hover_text("Set position to gaze")
                        .clicked()
                    {
                        self.start = pos;
                    }
                    ui.label(prettify_distance(d));
                });

                ui.end_row();

                input_float3!(ui, format!("{ICON_ALPHA_B_BOX} End "), &mut self.end);
                ui.horizontal(|ui| {
                    if ui
                        .button(ICON_CAMERA_CONTROL.to_string())
                        .on_hover_text("Set position to camera")
                        .clicked()
                    {
                        self.end = camera.position();
                    }

                    if ui
                        .add_enabled(
                            d.is_finite(),
                            Button::new(if d.is_finite() {
                                ICON_EYE_ARROW_RIGHT_OUTLINE.to_string()
                            } else {
                                ICON_EYE_OFF_OUTLINE.to_string()
                            }),
                        )
                        .on_hover_text("Set position to gaze")
                        .clicked()
                    {
                        self.end = pos;
                    }
                });
            });

        ui.horizontal(|ui| {
            ui.strong("Scale");
            ui.add(
                egui::DragValue::new(&mut self.scale)
                    .speed(0.1)
                    .range(0f32..=100f32)
                    .min_decimals(2)
                    .max_decimals(2),
            )
        });

        ui.horizontal(|ui| {
            ui.strong("Marker Interval");
            ui.add(
                egui::DragValue::new(&mut self.marker_interval)
                    .speed(0.1)
                    .range(0f32..=f32::INFINITY)
                    .min_decimals(2)
                    .max_decimals(2)
                    .suffix(" m"),
            )
        });
        ui.checkbox(&mut self.show_individual_axis, "Show individual axis");

        ui.horizontal(|ui| {
            ui.strong("Length:");
            ui.label(prettify_distance(self.length()));
        });

        if self.marker_interval > 0.0 {
            ui.horizontal(|ui| {
                ui.strong("Length remainder at end:");
                ui.label(prettify_distance(self.length() % self.marker_interval));
            });
        }

        ui.separator();

        ui.horizontal(|ui| {
            color_edit_button_rgba(ui, &mut self.color, Alpha::OnlyBlend).context_menu(|ui| {
                ui.checkbox(&mut self.rainbow, "Rainbow mode");
            });

            ui.label("Color");
        });
    }
}

impl ComponentPanel for Sphere {
    fn inspector_name() -> &'static str {
        "Sphere"
    }

    fn inspector_icon() -> char {
        Sphere::icon().char()
    }

    fn show_inspector_ui(
        &mut self,
        _: &mut Scene,
        _: &mut Commands<'_, '_>,
        e: EntityRef<'_>,
        ui: &mut egui::Ui,
        _resources: &AppResources,
    ) {
        if !e.contains::<Transform>() {
            ui.label(format!(
                "{} This entity has no transform component",
                ICON_ALERT
            ));
        }

        ui.horizontal(|ui| {
            ui.strong("Detail");
            ui.add(
                egui::DragValue::new(&mut self.detail)
                    .speed(0.1)
                    .range(2..=32),
            )
        });

        ui.horizontal(|ui| {
            color_edit_button_rgba(ui, &mut self.color, Alpha::OnlyBlend).context_menu(|ui| {
                ui.checkbox(&mut self.rainbow, "Rainbow mode");
            });

            ui.label("Color");
        });
    }
}

impl ComponentPanel for Beacon {
    fn inspector_name() -> &'static str {
        "Beacon"
    }

    fn inspector_icon() -> char {
        Beacon::icon().char()
    }

    fn show_inspector_ui(
        &mut self,
        _: &mut Scene,
        _: &mut Commands<'_, '_>,
        e: EntityRef<'_>,
        ui: &mut egui::Ui,
        resources: &AppResources,
    ) {
        if !e.contains::<Transform>() {
            ui.label(format!(
                "{} This entity has no transform component",
                ICON_ALERT
            ));
        }

        ui.horizontal(|ui| {
            ui.strong("Distance after travel: ");
            ui.add(
                egui::DragValue::new(&mut self.distance)
                    .speed(0.1)
                    .range(0f32..=f32::INFINITY)
                    .min_decimals(2)
                    .max_decimals(2)
                    .suffix(" m"),
            )
        });

        ui.horizontal(|ui| {
            ui.strong("Duration of travel: ");
            ui.add(
                egui::DragValue::new(&mut self.travel_time)
                    .speed(0.1)
                    .range(0f32..=60.0)
                    .min_decimals(2)
                    .max_decimals(2)
                    .suffix(" s"),
            )
        });

        ui.horizontal(|ui| {
            ui.strong("Blink Frequency");
            ui.add(
                egui::DragValue::new(&mut self.freq)
                    .speed(0.1)
                    .range(0.0..=20.0),
            )
        });

        ui.horizontal(|ui| {
            color_edit_button_rgba(ui, &mut self.color, Alpha::Opaque);

            ui.label("Color");
        });

        ui.separator();

        let mut camera = resources.get_mut::<Camera>();
        if let Some(transform) = e.get::<Transform>() {
            ui.label(format!(
                "Distance to Beacon: {:.2} m",
                (transform.translation - camera.position()).length()
            ));

            ui.horizontal(|ui| {
                if ui.button(ICON_MAP_MARKER.to_string()).clicked() {
                    camera.tween = Some(Tween::new(
                        |x| x,
                        Some((
                            camera.position(),
                            transform.translation - camera.forward() * self.distance,
                        )),
                        None,
                        self.travel_time,
                    ));
                }
                ui.label("Go to Beacon Location");
            });

            // TODO(cohae): Reimplement tween rotation
            ui.horizontal(|ui| {
                if ui.button(ICON_CAMERA.to_string()).clicked() {
                    camera.tween = Some(Tween::new(
                        |x| x,
                        None,
                        Some((
                            camera.view_angle(),
                            camera.get_look_angle(transform.translation),
                        )),
                        self.travel_time,
                    ));
                }
                ui.label("Look at Beacon Location");
            });
        }
    }
}

impl ComponentPanel for Route {
    fn inspector_name() -> &'static str {
        "Route"
    }

    fn inspector_icon() -> char {
        Route::icon().char()
    }

    fn show_inspector_ui(
        &mut self,
        scene: &mut Scene,
        cmd: &mut Commands<'_, '_>,
        e: EntityRef<'_>,
        ui: &mut egui::Ui,
        resources: &AppResources,
    ) {
        let camera = resources.get_mut::<Camera>();
        ui.horizontal(|ui| {
            color_edit_button_rgba(ui, &mut self.color, Alpha::Opaque);

            ui.label("Color");
        });

        ui.horizontal(|ui| {
            ui.strong("Speed Multiplier");
            ui.add(
                egui::DragValue::new(&mut self.speed_multiplier)
                    .speed(0.1)
                    .range(0.01f32..=30f32)
                    .min_decimals(2)
                    .max_decimals(2),
            )
        });

        ui.horizontal(|ui| {
            ui.strong("Scale");
            ui.add(
                egui::DragValue::new(&mut self.scale)
                    .speed(0.1)
                    .range(0f32..=100f32)
                    .min_decimals(2)
                    .max_decimals(2),
            )
        });

        ui.horizontal(|ui| {
            ui.strong("Marker Interval");
            ui.add(
                egui::DragValue::new(&mut self.marker_interval)
                    .speed(0.1)
                    .range(0f32..=f32::INFINITY)
                    .min_decimals(2)
                    .max_decimals(2)
                    .suffix(" m"),
            )
        });

        let old_value = self.show_all;
        ui.checkbox(&mut self.show_all, "Show nodes in all maps");
        if old_value != self.show_all {
            self.fixup_visiblity(scene, cmd, e.id());
        }

        ui.separator();

        ui.horizontal(|ui| {
            if ui
                .button(format!("{}", ICON_MAP_MARKER_PLUS))
                .on_hover_text("Hotkey: +")
                .clicked()
                || ui.input_mut(|i| i.consume_shortcut(&SHORTCUT_ADD_ROUTE_NODE_NEXT))
            {
                let node = cmd
                    .spawn(RouteNode::make_budle(
                        e.id(),
                        RouteNodeHolder {
                            pos: camera.position(),
                            map_hash: scene.get_map_hash(),
                            ..Default::default()
                        },
                    ))
                    .id();
                if let Some(mut children) = e.get_mut::<Children>() {
                    children.0.push(node);
                }
            };
            ui.label("Add Node to end of Route");
        });

        ui.separator();

        if ui
            .button(format!("{} Copy route command", ICON_CLIPBOARD,))
            .clicked()
        {
            let command = self.get_command(scene, e.id());
            ui.output_mut(|o| o.copied_text = command);
        }

        if ui
            .button(format!("{} Traverse Path", ICON_MAP_MARKER_PATH))
            .clicked()
        {
            resources
                .get_mut::<ActionList>()
                .add_action(FollowAction::new(e.id(), None));
        }

        ui.separator();
    }
}

impl ComponentPanel for RouteNode {
    fn inspector_name() -> &'static str {
        "Route Node"
    }

    fn inspector_icon() -> char {
        RouteNode::icon().char()
    }

    fn show_inspector_ui(
        &mut self,
        scene: &mut Scene,
        cmd: &mut Commands<'_, '_>,
        e: EntityRef<'_>,
        ui: &mut egui::Ui,
        resources: &AppResources,
    ) {
        if !e.contains::<Transform>() {
            ui.label(format!(
                "{} This entity has no transform component",
                ICON_ALERT
            ));
        }
        if !e.contains::<Parent>() {
            ui.label(format!("{} This Node has no associated Route", ICON_ALERT));
            return;
        }
        let Some(parent) = e.get::<Parent>() else {
            return;
        };
        let Some(node_pos) = e.get::<Transform>() else {
            return;
        };

        let mut camera = resources.get_mut::<Camera>();

        ui.checkbox(&mut self.is_teleport, "This node is teleported to");

        ui.separator();

        ui.horizontal(|ui| {
            if ui
                .button(format!("{}{}", ICON_ARROW_LEFT, ICON_MAP_MARKER_PLUS))
                .on_hover_text("Hotkey: -")
                .clicked()
                || ui.input_mut(|i| i.consume_shortcut(&SHORTCUT_ADD_ROUTE_NODE_PREV))
            {
                if let Some(mut children) = scene.entity(parent.0).get_mut::<Children>() {
                    let index = children
                        .0
                        .iter()
                        .position(|&ent| ent == e.id())
                        .unwrap_or(children.0.len());
                    let node = cmd
                        .spawn(RouteNode::make_budle(
                            parent.0,
                            RouteNodeHolder {
                                pos: camera.position(),
                                map_hash: scene.get_map_hash(),
                                ..Default::default()
                            },
                        ))
                        .id();
                    children.0.insert(index, node);
                    resources.get_mut::<SelectedEntity>().select(node);
                }
            };
            ui.label("Add Node before this one");
        });

        ui.horizontal(|ui| {
            if ui
                .button(format!("{}{}", ICON_MAP_MARKER_PLUS, ICON_ARROW_RIGHT))
                .on_hover_text("Hotkey: +")
                .clicked()
                || ui.input_mut(|i| i.consume_shortcut(&SHORTCUT_ADD_ROUTE_NODE_NEXT))
            {
                if let Some(mut children) = scene.entity(parent.0).get_mut::<Children>() {
                    let index = children
                        .0
                        .iter()
                        .position(|&ent| ent == e.id())
                        .unwrap_or(children.0.len());
                    let node = cmd
                        .spawn(RouteNode::make_budle(
                            parent.0,
                            RouteNodeHolder {
                                pos: camera.position(),
                                map_hash: scene.get_map_hash(),
                                ..Default::default()
                            },
                        ))
                        .id();
                    children.0.insert(index + 1, node);
                    resources.get_mut::<SelectedEntity>().select(node);
                }
            };
            ui.label("Add Node after this one");
        });

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button(ICON_MAP_MARKER.to_string()).clicked() {
                camera.tween = Some(Tween::new(
                    |x| x,
                    Some((
                        camera.position(),
                        node_pos.translation - camera.forward() * 5.0,
                    )),
                    None,
                    1.0,
                ));
            }
            ui.label("Go to Node Location");
        });

        ui.horizontal(|ui| {
            if ui.button(format!("{}", ICON_MAP_MARKER_PATH)).clicked() {
                resources
                    .get_mut::<ActionList>()
                    .add_action(FollowAction::new(parent.0, Some(e.id())));
            }
            ui.label("Traverse Route starting at this Node");
        });
    }
}
