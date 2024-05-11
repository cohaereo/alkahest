use alkahest_renderer::{
    camera::{tween::Tween, Camera},
    ecs::{
        transform::Transform,
        utility::{Beacon, Ruler, Sphere},
        Scene,
    },
    icons::{
        ICON_ALERT, ICON_ALPHA_A_BOX, ICON_ALPHA_B_BOX, ICON_CAMERA_CONTROL,
        ICON_EYE_ARROW_RIGHT_OUTLINE, ICON_EYE_OFF_OUTLINE, ICON_MAP_MARKER, ICON_RULER_SQUARE,
        ICON_SIGN_POLE, ICON_SPHERE,
    },
    renderer::RendererShared,
};
use egui::{
    color_picker::{color_edit_button_rgba, Alpha},
    Button,
};
use hecs::EntityRef;

use crate::{
    gui::inspector::ComponentPanel, input_float3, resources::Resources,
    util::text::prettify_distance,
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
            // ui.horizontal(|ui| {
            //     if ui.button(ICON_CAMERA.to_string()).clicked() {
            //         camera.tween = Some(Tween::new(
            //             |x| x,
            //             None,
            //             Some((
            //                 camera.orientation(),
            //                 camera.get_look_angle(transform.translation),
            //             )),
            //             self.travel_time,
            //         ));
            //     }
            //     ui.label("Look at Beacon Location");
            // });
        }
    }
}
