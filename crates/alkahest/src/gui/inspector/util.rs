use alkahest_renderer::{
    camera::{get_look_angle, tween::Tween, Camera},
    ecs::{
        transform::Transform,
        utility::{Beacon, Route, RouteNode, Ruler, Sphere},
        Scene,
    },
    icons::{
        ICON_ALERT, ICON_ALPHA_A_BOX, ICON_ALPHA_B_BOX, ICON_CAMERA, ICON_CAMERA_CONTROL,
        ICON_CLIPBOARD, ICON_DELETE, ICON_EYE_ARROW_RIGHT_OUTLINE, ICON_EYE_OFF_OUTLINE,
        ICON_MAP_MARKER, ICON_MAP_MARKER_PATH, ICON_MAP_MARKER_PLUS, ICON_RULER_SQUARE,
        ICON_SIGN_POLE, ICON_SPHERE, ICON_TAG,
    },
    renderer::RendererShared,
};
use destiny_pkg::TagHash;
use egui::{
    color_picker::{color_edit_button_rgba, Alpha},
    Button,
};
use glam::Vec3;
use hecs::EntityRef;
use serde_yaml::value::Tag;

use crate::{
    gui::inspector::ComponentPanel,
    input_float3,
    maplist::MapList,
    resources::Resources,
    util::{
        action::{ActionList, ActivitySwapAction, MapSwapAction, TweenAction},
        text::prettify_distance,
    },
};

impl ComponentPanel for Ruler {
    fn inspector_name() -> &'static str {
        "Ruler"
    }

    fn inspector_icon() -> char {
        ICON_RULER_SQUARE
    }

    fn has_inspector_ui() -> bool {
        true
    }

    fn show_inspector_ui(
        &mut self,
        _: &Scene,
        _: EntityRef<'_>,
        ui: &mut egui::Ui,
        resources: &Resources,
        _: TagHash,
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
                    .clamp_range(0f32..=100f32)
                    .min_decimals(2)
                    .max_decimals(2),
            )
        });

        ui.horizontal(|ui| {
            ui.strong("Marker Interval");
            ui.add(
                egui::DragValue::new(&mut self.marker_interval)
                    .speed(0.1)
                    .clamp_range(0f32..=f32::INFINITY)
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
        ICON_SPHERE
    }

    fn has_inspector_ui() -> bool {
        true
    }

    fn show_inspector_ui(
        &mut self,
        _: &Scene,
        e: EntityRef<'_>,
        ui: &mut egui::Ui,
        _resources: &Resources,
        _: TagHash,
    ) {
        if !e.has::<Transform>() {
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
                    .clamp_range(2..=32),
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
        ICON_SIGN_POLE
    }

    fn has_inspector_ui() -> bool {
        true
    }

    fn show_inspector_ui(
        &mut self,
        _: &Scene,
        e: EntityRef<'_>,
        ui: &mut egui::Ui,
        resources: &Resources,
        _: TagHash,
    ) {
        if !e.has::<Transform>() {
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
                    .clamp_range(0f32..=f32::INFINITY)
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
                    .clamp_range(0f32..=60.0)
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
                    .clamp_range(0.0..=20.0),
            )
        });

        ui.horizontal(|ui| {
            color_edit_button_rgba(ui, &mut self.color, Alpha::Opaque);

            ui.label("Color");
        });

        ui.separator();

        let mut camera = resources.get_mut::<Camera>();
        if let Some(transform) = e.get::<&Transform>() {
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
        ICON_MAP_MARKER_PATH
    }

    fn has_inspector_ui() -> bool {
        true
    }

    fn show_inspector_ui(
        &mut self,
        _: &Scene,
        _: EntityRef<'_>,
        ui: &mut egui::Ui,
        resources: &Resources,
        current_hash: TagHash,
    ) {
        let mut camera = resources.get_mut::<Camera>();

        let (d, pos) = resources
            .get::<RendererShared>()
            .data
            .lock()
            .gbuffers
            .depth_buffer_distance_pos_center(&camera);
        let mut new_node: Option<(usize, RouteNode)> = None;
        let mut del_node: Option<usize> = None;
        let mut traverse_from: Option<usize> = None;
        egui::ScrollArea::vertical()
            .max_height(ui.available_height() - ui.spacing().interact_size.y * 15.0)
            .show(ui, |ui| {
                for (i, node) in self.path.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        if ui
                            .button(format!("{}", ICON_MAP_MARKER_PLUS))
                            .on_hover_text("Insert Node")
                            .clicked()
                        {
                            new_node = Some((
                                i,
                                RouteNode {
                                    pos: camera.position(),
                                    map_hash: Some(current_hash),
                                    is_teleport: false,
                                    label: None,
                                },
                            ));
                        };
                        ui.add(egui::Separator::default().horizontal());
                    });

                    egui::Grid::new(format!("route_name_{}", i))
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            if ui
                                .button(format!("{}", ICON_DELETE))
                                .on_hover_text("Delete Node")
                                .clicked()
                            {
                                del_node = Some(i);
                            };
                            ui.horizontal(|ui| {
                                if ui
                                    .button(ICON_MAP_MARKER.to_string())
                                    .on_hover_text("Go to Node Location")
                                    .clicked()
                                {
                                    camera.tween = Some(Tween::new(
                                        |x| x,
                                        Some((
                                            camera.position(),
                                            node.pos - camera.forward() * 0.5,
                                        )),
                                        None,
                                        0.7,
                                    ));
                                }
                                if let Some(label) = node.label.as_mut() {
                                    egui::TextEdit::singleline(label);
                                } else {
                                    ui.label(format!("Node {}", i + 1));
                                    if ui
                                        .button(ICON_TAG.to_string())
                                        .on_hover_text("Add label")
                                        .clicked()
                                    {
                                        node.label = Some(format!("Node {}", i + 1));
                                    }
                                }
                                if ui
                                    .button(format!("{}", ICON_MAP_MARKER_PATH))
                                    .on_hover_text("Traverse Path from this node")
                                    .clicked()
                                {
                                    traverse_from = Some(i)
                                }
                            });
                        });
                    egui::Grid::new(format!("route_position_{}", i))
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            input_float3!(ui, "Position", &mut node.pos);

                            ui.horizontal(|ui| {
                                if ui
                                    .button(ICON_CAMERA_CONTROL.to_string())
                                    .on_hover_text("Set position to camera")
                                    .clicked()
                                {
                                    node.pos = camera.position();
                                    node.map_hash = Some(current_hash);
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
                                    node.pos = pos;
                                    node.map_hash = Some(current_hash);
                                }
                                ui.label(prettify_distance(d));
                            });
                        });
                    ui.checkbox(&mut node.is_teleport, "This node is teleported to");
                }
            });
        ui.horizontal(|ui| {
            if ui
                .button(format!("{}", ICON_MAP_MARKER_PLUS))
                .on_hover_text("Add Node")
                .clicked()
            {
                self.path.push(RouteNode {
                    pos: camera.position(),
                    map_hash: Some(current_hash),
                    is_teleport: false,
                    label: None,
                });
            };
            ui.add(egui::Separator::default().horizontal());
        });

        del_node.map(|pos| self.path.remove(pos));
        if let Some((pos, node)) = new_node {
            self.path.insert(pos, node)
        }
        ui.separator();

        if ui
            .button(format!("{} Traverse Path", ICON_MAP_MARKER_PATH))
            .clicked()
        {
            traverse_from = Some(0)
        }
        ui.horizontal(|ui| {
            ui.strong("Speed Multiplier");
            ui.add(
                egui::DragValue::new(&mut self.speed_multiplier)
                    .speed(0.1)
                    .clamp_range(0.01f32..=30f32)
                    .min_decimals(2)
                    .max_decimals(2),
            )
        });
        if let Some(start_index) = traverse_from {
            let camera_offset = Vec3::Z;
            let mut action_list = resources.get_mut::<ActionList>();
            const DEGREES_PER_SEC: f32 = 360.0;
            const METERS_PER_SEC: f32 = 18.0;
            action_list.clear_actions();

            if let Some(start_pos) = self.path.get(start_index) {
                let mut old_pos = start_pos.pos + camera_offset;
                let mut old_orient = camera.get_look_angle(old_pos);
                action_list.add_action(TweenAction::new(
                    |x| x,
                    Some((camera.position(), old_pos)),
                    Some((camera.view_angle(), old_orient)),
                    1.0,
                ));

                if let Some(hash) = start_pos.map_hash {
                    action_list.add_action(MapSwapAction::new(hash));
                }

                for node in self.path.iter().skip(start_index + 1) {
                    let new_pos = node.pos + camera_offset;
                    let new_orient = get_look_angle(old_orient, old_pos, new_pos);
                    //TODO Not sure why this isn't working right
                    // let angle_dif = get_look_angle_difference(old_orient, old_pos, new_pos);
                    // Using a silly approximation to look ok.
                    let angle_delta = (old_orient - new_orient).abs();
                    let angle_dif = (angle_delta.x % 360.0).max(angle_delta.y % 360.0);
                    action_list.add_action(TweenAction::new(
                        |x| x,
                        None,
                        Some((old_orient, new_orient)),
                        angle_dif / (DEGREES_PER_SEC * self.speed_multiplier),
                    ));
                    old_orient = new_orient;
                    action_list.add_action(TweenAction::new(
                        |x| x,
                        Some((old_pos, new_pos)),
                        None,
                        if node.is_teleport {
                            self.scale * 0.1
                        } else {
                            self.scale * old_pos.distance(new_pos)
                                / (METERS_PER_SEC * self.speed_multiplier)
                        },
                    ));
                    if let Some(hash) = node.map_hash {
                        action_list.add_action(MapSwapAction::new(hash));
                    }
                    old_pos = new_pos;
                }
            }
        }
        ui.checkbox(&mut self.show_all, "Show nodes in all maps");

        if ui
            .button(format!("{} Copy route command", ICON_CLIPBOARD,))
            .clicked()
        {
            let mut command = String::from("route");
            if let Some(hash) = self.activity_hash.as_ref() {
                command = format!("{} hash {}", command, hash.0);
            }
            for node in self.path.iter() {
                command = format!(
                    "{} node {} {} {}{}{}{}",
                    command,
                    node.pos[0],
                    node.pos[1],
                    node.pos[2],
                    if node.is_teleport { " tp" } else { "" },
                    node.map_hash
                        .map_or(String::new(), |h| { format!(" hash {}", h.0) }),
                    node.label.as_ref().map_or(String::new(), |s| {
                        format!(" label {}", s.replace('\\', r"\\").replace(' ', r"\s"))
                    })
                );
            }

            ui.output_mut(|o| o.copied_text = command);
        }

        ui.horizontal(|ui| {
            ui.strong("Scale");
            ui.add(
                egui::DragValue::new(&mut self.scale)
                    .speed(0.1)
                    .clamp_range(0f32..=100f32)
                    .min_decimals(2)
                    .max_decimals(2),
            )
        });

        ui.horizontal(|ui| {
            ui.strong("Marker Interval");
            ui.add(
                egui::DragValue::new(&mut self.marker_interval)
                    .speed(0.1)
                    .clamp_range(0f32..=f32::INFINITY)
                    .min_decimals(2)
                    .max_decimals(2)
                    .suffix(" m"),
            )
        });

        ui.separator();

        ui.horizontal(|ui| {
            color_edit_button_rgba(ui, &mut self.color, Alpha::Opaque);

            ui.label("Color");
        });
    }
}
