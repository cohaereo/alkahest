use egui::{Button, Color32, FontId, RichText, Widget};
use glam::{Quat, Vec3};
use hecs::{Entity, EntityRef};

use super::{
    components::{
        Beacon, EntityModel, EntityWorldId, Global, Label, Mutable, ResourcePoint, Ruler, Sphere,
        StaticInstances, Visible,
    },
    resolve_entity_icon, resolve_entity_name,
    tags::{insert_tag, remove_tag, EntityTag, Tags},
    transform::{OriginalTransform, Transform},
    Scene,
};
use crate::{
    camera::FpsCamera,
    ecs::transform::TransformFlags,
    hotkeys::{SHORTCUT_DELETE, SHORTCUT_HIDE},
    icons::{
        ICON_ALERT, ICON_ALPHA_A_BOX, ICON_ALPHA_B_BOX, ICON_AXIS_ARROW, ICON_CAMERA,
        ICON_CAMERA_CONTROL, ICON_CUBE_OUTLINE, ICON_DELETE, ICON_EYE,
        ICON_EYE_ARROW_RIGHT_OUTLINE, ICON_EYE_OFF, ICON_EYE_OFF_OUTLINE, ICON_HELP,
        ICON_IDENTIFIER, ICON_MAP_MARKER, ICON_RADIUS_OUTLINE, ICON_RESIZE, ICON_ROTATE_ORBIT,
        ICON_RULER_SQUARE, ICON_SIGN_POLE, ICON_SPHERE, ICON_TAG,
    },
    render::tween::Tween,
    resources::Resources,
    util::{
        text::{prettify_distance, split_pascal_case},
        BoolExts as _,
    },
    RendererShared,
};

pub fn show_inspector_panel(
    ui: &mut egui::Ui,
    scene: &mut Scene,
    cmd: &mut hecs::CommandBuffer,
    ent: Entity,
    resources: &Resources,
) {
    let Ok(e) = scene.entity(ent) else {
        return;
    };

    ui.horizontal(|ui| {
        let visible = if let Some(vis) = e.get::<&Visible>() {
            vis.0
        } else {
            true
        };

        if e.has::<Mutable>()
            && (ui
                .button(RichText::new(ICON_DELETE).size(24.0).strong())
                .clicked()
                || ui.input_mut(|i| i.consume_shortcut(&SHORTCUT_DELETE)))
        {
            cmd.despawn(ent);
        }

        if ui
            .button(
                RichText::new(if visible { ICON_EYE } else { ICON_EYE_OFF })
                    .size(24.0)
                    .strong(),
            )
            .clicked()
            || ui.input_mut(|i| i.consume_shortcut(&SHORTCUT_HIDE))
        {
            if let Some(mut vis) = e.get::<&mut Visible>() {
                vis.0 = !visible;
            } else {
                cmd.insert_one(ent, Visible(!visible));
            }
        }

        let title = format!(
            "{} {}",
            resolve_entity_icon(e).unwrap_or(ICON_HELP),
            resolve_entity_name(e, true)
        );

        if e.has::<Mutable>() {
            if let Some(mut label) = e.get::<&mut Label>() {
                egui::TextEdit::singleline(&mut label.0)
                    .font(FontId::proportional(22.0))
                    .ui(ui);
            } else {
                ui.label(RichText::new(title).size(24.0).strong());
                if ui
                    .button(RichText::new(ICON_TAG.to_string()).size(24.0).strong())
                    .on_hover_text("Add label")
                    .clicked()
                {
                    cmd.insert_one(ent, Label(resolve_entity_name(e, false)));
                }
            }
        } else {
            ui.label(RichText::new(title).size(24.0).strong());
        }
    });
    ui.separator();

    if let Some(tags) = e.get::<&Tags>() {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Tags: ").color(Color32::WHITE).strong());
            tags.ui_chips(ui);
        });
        ui.separator();
    }

    let mut global = e.get::<&Global>().map_or(false, |g| g.0);
    let mut global_changed = false;
    if e.has::<Mutable>() {
        if ui.checkbox(&mut global, "Show in all Maps").clicked() {
            global_changed = true;
            if let Some(mut g) = e.get::<&mut Global>() {
                g.0 = global;
            } else {
                cmd.insert_one(ent, Global(global));
            }
        };
        ui.separator();
    }
    show_inspector_components(ui, e, resources);

    if global_changed {
        if global {
            insert_tag(scene, ent, EntityTag::Global);
        } else {
            remove_tag(scene, ent, EntityTag::Global);
        }
    }
}

fn show_inspector_components(ui: &mut egui::Ui, e: EntityRef<'_>, resources: &Resources) {
    if let Some(mut t) = e.get::<&mut Transform>() {
        inspector_component_frame(ui, "Transform", ICON_AXIS_ARROW, |ui| {
            t.show_inspector_ui(e, ui, resources);
            if let Some(ot) = e.get::<&OriginalTransform>() {
                // Has the entity moved from it's original position?
                let has_moved = *t != ot.0;
                ui.add_enabled_ui(has_moved, |ui: &mut egui::Ui| {
					if ui.button("Reset to original")
						.on_hover_text("This object has an original transform defined.\nClicking this button will reset the current transform back  to the original")
						.clicked()
					{
						*t = ot.0;
					}
				});
            }
        });
    }

    macro_rules! component_views {
		($($component:ty),+) => {
			$(
				if let Some(mut component) = e.get::<&mut $component>() {
					inspector_component_frame(ui, <$component>::inspector_name(), <$component>::inspector_icon(), |ui| {
						component.show_inspector_ui(e, ui, resources);
					});
				}
			)*
		};
	}

    component_views!(
        ResourcePoint,
        EntityModel,
        StaticInstances,
        // HavokShape,
        EntityWorldId,
        Ruler,
        Sphere,
        Beacon
    );
}

fn inspector_component_frame(
    ui: &mut egui::Ui,
    title: &str,
    icon: char,
    add_body: impl FnOnce(&mut egui::Ui),
) {
    egui::CollapsingHeader::new(RichText::new(format!("{icon} {title}")).strong())
        .show(ui, add_body);

    ui.separator();
}

// TODO(cohae): Move these to a util module together with input_float4
macro_rules! input_float3 {
    ($ui:expr, $label:expr, $v:expr) => {{
        $ui.label($label);
        $ui.horizontal(|ui| {
            let c0 = ui
                .add(
                    egui::DragValue::new(&mut $v.x)
                        .speed(0.1)
                        .prefix("x: ")
                        .min_decimals(2)
                        .max_decimals(2),
                )
                .changed();
            let c1 = ui
                .add(
                    egui::DragValue::new(&mut $v.y)
                        .speed(0.1)
                        .prefix("y: ")
                        .min_decimals(2)
                        .max_decimals(2),
                )
                .changed();
            let c2 = ui
                .add(
                    egui::DragValue::new(&mut $v.z)
                        .speed(0.1)
                        .prefix("z: ")
                        .min_decimals(2)
                        .max_decimals(2),
                )
                .changed();

            c0 || c1 || c2
        })
    }};
}

pub(super) trait ComponentPanel {
    fn inspector_name() -> &'static str;
    fn inspector_icon() -> char {
        ICON_CUBE_OUTLINE
    }

    // TODO(cohae): Not the most ergonomic thing ever
    fn has_inspector_ui() -> bool {
        false
    }

    fn show_inspector_ui(&mut self, _: EntityRef<'_>, _: &mut egui::Ui, _: &Resources) {}
}

impl ComponentPanel for Transform {
    fn inspector_name() -> &'static str {
        "Transform"
    }

    fn inspector_icon() -> char {
        ICON_AXIS_ARROW
    }

    fn has_inspector_ui() -> bool {
        true
    }

    fn show_inspector_ui(&mut self, _: EntityRef<'_>, ui: &mut egui::Ui, resources: &Resources) {
        let mut rotation_euler: Vec3 = self.rotation.to_euler(glam::EulerRot::XYZ).into();
        rotation_euler.x = rotation_euler.x.to_degrees();
        rotation_euler.y = rotation_euler.y.to_degrees();
        rotation_euler.z = rotation_euler.z.to_degrees();

        let mut rotation_changed = false;
        egui::Grid::new("transform_input_grid")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                if !self.flags.contains(TransformFlags::IGNORE_TRANSLATION) {
                    input_float3!(
                        ui,
                        format!("{ICON_AXIS_ARROW} Translation"),
                        &mut self.translation
                    );

                    ui.horizontal(|ui| {
                        if let Some(camera) = resources.get::<FpsCamera>() {
                            if ui
                                .button(ICON_CAMERA_CONTROL.to_string())
                                .on_hover_text("Set position to camera")
                                .clicked()
                            {
                                self.translation = camera.position;
                            }
                            if let Some(renderer) = resources.get::<RendererShared>() {
                                let (d, pos) = renderer
                                    .read()
                                    .gbuffer
                                    .depth_buffer_distance_pos_center(&camera);
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
                                    self.translation = pos;
                                }
                                ui.label(prettify_distance(d));
                            }
                        }
                    });
                    ui.end_row();
                }
                if !self.flags.contains(TransformFlags::IGNORE_ROTATION) {
                    rotation_changed = input_float3!(
                        ui,
                        format!("{ICON_ROTATE_ORBIT} Rotation"),
                        &mut rotation_euler
                    )
                    .inner;
                    ui.end_row();
                }
                if !self.flags.contains(TransformFlags::IGNORE_SCALE) {
                    if self.flags.contains(TransformFlags::SCALE_IS_RADIUS) {
                        ui.label(format!("{ICON_RADIUS_OUTLINE} Radius"));
                        ui.add(
                            egui::DragValue::new(&mut self.scale.x)
                                .speed(0.1)
                                .clamp_range(0f32..=f32::INFINITY)
                                .min_decimals(2)
                                .max_decimals(2),
                        );

                        if let Some(camera) = resources.get::<FpsCamera>() {
                            if ui
                                .button(ICON_RADIUS_OUTLINE.to_string())
                                .on_hover_text("Set radius to camera")
                                .clicked()
                            {
                                self.scale = Vec3::splat(
                                    (self.translation - camera.position).length().max(0.1),
                                );
                            }
                        }
                    } else {
                        input_float3!(ui, format!("{ICON_RESIZE} Scale"), &mut self.scale);
                    }
                    ui.end_row();
                }
            });

        if rotation_changed {
            self.rotation = Quat::from_euler(
                glam::EulerRot::XYZ,
                rotation_euler.x.to_radians(),
                rotation_euler.y.to_radians(),
                rotation_euler.z.to_radians(),
            );
        }
    }
}

impl ComponentPanel for EntityWorldId {
    fn inspector_name() -> &'static str {
        "World ID"
    }

    fn inspector_icon() -> char {
        ICON_IDENTIFIER
    }

    fn has_inspector_ui() -> bool {
        true
    }

    fn show_inspector_ui(&mut self, _: EntityRef<'_>, ui: &mut egui::Ui, _: &Resources) {
        ui.label(format!("World ID: 0x{:016X}", self.0));
    }
}

impl ComponentPanel for ResourcePoint {
    fn inspector_name() -> &'static str {
        "Map Resource"
    }

    fn inspector_icon() -> char {
        ICON_MAP_MARKER
    }

    fn has_inspector_ui() -> bool {
        true
    }

    fn show_inspector_ui(&mut self, _: EntityRef<'_>, ui: &mut egui::Ui, _: &Resources) {
        ui.horizontal(|ui| {
            ui.strong("Entity:");
            ui.label(self.entity.to_string());
        });
        ui.horizontal(|ui| {
            ui.strong("Origin:");
            ui.label(split_pascal_case(&self.origin.to_string()));
        });
        ui.horizontal(|ui| {
            ui.strong("Has havok data?:");
            ui.label(self.has_havok_data.yes_no());
        });
        ui.horizontal(|ui| {
            let c = self.resource.debug_color();
            let color = egui::Color32::from_rgb(c[0], c[1], c[2]);

            ui.strong("Type: ");
            ui.label(
                RichText::new(format!(
                    "{} {}",
                    self.resource.debug_icon(),
                    self.resource.debug_id()
                ))
                .color(color),
            );
        });
        ui.separator();
        ui.label(RichText::new(self.resource.debug_string()).italics());
    }
}

impl ComponentPanel for EntityModel {
    fn inspector_name() -> &'static str {
        "Entity Model"
    }

    fn inspector_icon() -> char {
        ICON_CUBE_OUTLINE
    }

    fn has_inspector_ui() -> bool {
        true
    }

    fn show_inspector_ui(&mut self, _: EntityRef<'_>, ui: &mut egui::Ui, _: &Resources) {
        ui.horizontal(|ui| {
            ui.strong("Tag:");
            ui.label(format!("{}", self.2));
        });
    }
}

impl ComponentPanel for StaticInstances {
    fn inspector_name() -> &'static str {
        "Static Instance Group"
    }

    fn inspector_icon() -> char {
        ICON_CUBE_OUTLINE
    }

    fn has_inspector_ui() -> bool {
        true
    }

    fn show_inspector_ui(&mut self, _: EntityRef<'_>, ui: &mut egui::Ui, _: &Resources) {
        ui.horizontal(|ui| {
            ui.strong("Mesh tag:");
            ui.label(self.1.to_string());
        });
        ui.horizontal(|ui| {
            ui.strong("Instance count:");
            ui.label(format!("{}", self.0.instance_count));
        });
    }
}

// impl ComponentPanel for HavokShape {
//     fn inspector_name() -> &'static str {
//         "Havok Shape"
//     }

//     fn inspector_icon() -> char {
//         ICON_HAZARD_LIGHTS
//     }

//     fn has_inspector_ui() -> bool {
//         true
//     }

//     fn show_inspector_ui(&mut self, ui: &mut egui::Ui) {
//         ui.horizontal(|ui| {
//             ui.strong("Havok tag:");
//             ui.label(self.0.to_string());
//         });
//         ui.horizontal(|ui| {
//             ui.strong("Has debugshape:");
//             ui.label(self.1.is_some().yes_no());
//         });
//     }
// }

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

    fn show_inspector_ui(&mut self, _: EntityRef<'_>, ui: &mut egui::Ui, resources: &Resources) {
        let camera = resources.get::<FpsCamera>().unwrap();
        egui::Grid::new("transform_input_grid")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                let (d, pos) = if let Some(renderer) = resources.get::<RendererShared>() {
                    renderer
                        .read()
                        .gbuffer
                        .depth_buffer_distance_pos_center(&camera)
                } else {
                    (0.0, camera.position)
                };
                input_float3!(ui, format!("{ICON_ALPHA_A_BOX} Start"), &mut self.start);
                ui.horizontal(|ui| {
                    if ui
                        .button(ICON_CAMERA_CONTROL.to_string())
                        .on_hover_text("Set position to camera")
                        .clicked()
                    {
                        self.start = camera.position;
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
                        self.end = camera.position;
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
            ui.color_edit_button_srgb(&mut self.color)
                .context_menu(|ui| {
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

    fn show_inspector_ui(&mut self, e: EntityRef<'_>, ui: &mut egui::Ui, _resources: &Resources) {
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
            ui.color_edit_button_srgba_unmultiplied(&mut self.color)
                .context_menu(|ui| {
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

    fn show_inspector_ui(&mut self, e: EntityRef<'_>, ui: &mut egui::Ui, resources: &Resources) {
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
            ui.color_edit_button_srgb(&mut self.color);

            ui.label("Color");
        });

        ui.separator();

        let mut camera = resources.get_mut::<FpsCamera>().unwrap();
        if let Some(transform) = e.get::<&Transform>() {
            ui.label(format!(
                "Distance to Beacon: {:.2} m",
                (transform.translation - camera.position).length()
            ));

            ui.horizontal(|ui| {
                if ui.button(ICON_MAP_MARKER.to_string()).clicked() {
                    camera.tween = Some(Tween::new(
                        |x| x,
                        Some((
                            camera.position,
                            transform.translation - camera.front * self.distance,
                        )),
                        None,
                        self.travel_time,
                    ));
                }
                ui.label("Go to Beacon Location");
            });
            ui.horizontal(|ui| {
                if ui.button(ICON_CAMERA.to_string()).clicked() {
                    camera.tween = Some(Tween::new(
                        |x| x,
                        None,
                        Some((
                            camera.orientation,
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
