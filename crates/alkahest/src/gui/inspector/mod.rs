mod light;
mod util;

use alkahest_data::map::{SLight, SShadowingLight};
use alkahest_renderer::{
    camera::{tween::Tween, Camera},
    ecs::{
        common::{EntityWorldId, Global, Hidden, Icon, Label, Mutable},
        dynamic_geometry::DynamicModelComponent,
        hierarchy::Parent,
        light::LightRenderer,
        resources::SelectedEntity,
        static_geometry::{StaticInstance, StaticInstances},
        tags::{insert_tag, remove_tag, EntityTag, Tags},
        transform::{OriginalTransform, Transform, TransformFlags},
        utility::{Beacon, Ruler, Sphere},
        Scene,
    },
    icons::ICON_LIGHTBULB_ON,
    renderer::RendererShared,
    util::color::Color,
};
use egui::{Align2, Button, Color32, FontId, RichText, Ui, Widget};
use glam::{Quat, Vec3};
use hecs::{Entity, EntityRef};
use winit::window::Window;

use crate::{
    gui::{
        chip::EcsTagsExt,
        context::{GuiCtx, GuiView, ViewResult},
        hotkeys::{SHORTCUT_DELETE, SHORTCUT_HIDE},
        icons::{
            ICON_ALERT, ICON_ALPHA_A_BOX, ICON_ALPHA_B_BOX, ICON_AXIS_ARROW, ICON_CAMERA_CONTROL,
            ICON_CUBE_OUTLINE, ICON_DELETE, ICON_EYE, ICON_EYE_ARROW_RIGHT_OUTLINE, ICON_EYE_OFF,
            ICON_EYE_OFF_OUTLINE, ICON_HELP, ICON_IDENTIFIER, ICON_MAP_MARKER, ICON_RADIUS_OUTLINE,
            ICON_RESIZE, ICON_ROTATE_ORBIT, ICON_RULER_SQUARE, ICON_SIGN_POLE, ICON_SPHERE,
            ICON_TAG,
        },
    },
    input_float3,
    maplist::MapList,
    resources::Resources,
    util::text::prettify_distance,
};

pub struct InspectorPanel;

impl GuiView for InspectorPanel {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &Window,
        resources: &Resources,
        _gui: &GuiCtx<'_>,
    ) -> Option<ViewResult> {
        let mut maps = resources.get_mut::<MapList>();

        if let Some(map) = maps.current_map_mut() {
            egui::Window::new("Inspector").show(ctx, |ui| {
                if let Some(ent) = resources.get::<SelectedEntity>().selected() {
                    show_inspector_panel(
                        ui,
                        &mut map.scene,
                        &mut map.command_buffer,
                        ent,
                        resources,
                    );
                } else {
                    ui.colored_label(Color32::WHITE, "No entity selected");
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::WHITE, "Select one using");
                        let p = ui.painter_at(ui.cursor());
                        let pos = ui.cursor().min;
                        ui.label("  ");

                        p.text(
                            pos,
                            Align2::LEFT_TOP,
                            "", // RMB button bg
                            FontId::proportional(ui.text_style_height(&egui::TextStyle::Body)),
                            Color32::from_rgb(0x33, 0x96, 0xda),
                        );

                        p.text(
                            pos,
                            Align2::LEFT_TOP,
                            "", // RMB button foreground
                            FontId::proportional(ui.text_style_height(&egui::TextStyle::Body)),
                            Color32::WHITE,
                        );
                    });
                }
            });
        }

        None
    }
}

pub fn resolve_entity_icon(e: EntityRef<'_>) -> Option<char> {
    macro_rules! icon_from_component_panels {
		($($component:ty),+) => {
			$(
				if e.has::<$component>() {
					return Some(<$component>::inspector_icon());
				}
			)*
		};
	}

    if let Some(rp) = e.get::<&Icon>() {
        return Some(rp.0);
    }

    icon_from_component_panels!(Beacon, Ruler, Sphere);

    None
}

pub fn resolve_entity_name(e: EntityRef<'_>, append_ent: bool) -> String {
    let postfix = if append_ent {
        format!(" (ent {})", e.entity().id())
    } else {
        String::new()
    };

    if let Some(label) = e.get::<&Label>() {
        format!("{}{postfix}", label.0)
    // } else if let Some(rp) = e.get::<&ResourcePoint>() {
    //     format!("{}{postfix}", split_pascal_case(rp.resource.debug_id()))
    } else {
        macro_rules! name_from_component_panels {
            ($($component:ty),+) => {
                $(
                    if e.has::<$component>() {
                        return format!("{}{postfix}", <$component>::inspector_name());
                    }
                )*
            };
        }

        name_from_component_panels!(Beacon, Ruler, Sphere);

        format!("ent {}", e.entity().id())
    }
}

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
        let visible = !e.has::<Hidden>();

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
            if visible {
                cmd.insert_one(ent, Hidden);
            } else {
                cmd.remove_one::<Hidden>(ent);
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

    let mut global = e.has::<Global>();
    let mut global_changed = false;
    if e.has::<Mutable>() {
        if ui.checkbox(&mut global, "Show in all Maps").changed() {
            global_changed = true;
            if global {
                cmd.insert_one(ent, Global);
            } else {
                cmd.remove_one::<Global>(ent);
            }
        };
        ui.separator();
    }
    show_inspector_components(ui, scene, e, resources);

    if global_changed {
        if global {
            insert_tag(scene, ent, EntityTag::Global);
        } else {
            remove_tag(scene, ent, EntityTag::Global);
        }
    }
}

fn show_inspector_components(
    ui: &mut egui::Ui,
    scene: &Scene,
    e: EntityRef<'_>,
    resources: &Resources,
) {
    if let Some(mut t) = e.get::<&mut Transform>() {
        inspector_component_frame(ui, "Transform", ICON_AXIS_ARROW, |ui| {
            t.show_inspector_ui(scene, e, ui, resources);
            if let Some(ot) = e.get::<&OriginalTransform>() {
                // Has the entity moved from it's original position?
                let has_moved = *t != ot.0;
                ui.add_enabled_ui(has_moved, |ui: &mut egui::Ui| {
                    if ui
                        .button("Reset to original")
                        .on_hover_text(
                            "This object has an original transform defined.\nClicking this button \
                             will reset the current transform back  to the original",
                        )
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
						component.show_inspector_ui(scene, e, ui, resources);
					});
				}
			)*
		};
	}

    component_views!(
        EntityWorldId,
        Ruler,
        Sphere,
        Beacon,
        DynamicModelComponent,
        LightRenderer
    );
}

fn inspector_component_frame(
    ui: &mut egui::Ui,
    title: &str,
    icon: char,
    add_body: impl FnOnce(&mut egui::Ui),
) {
    egui::CollapsingHeader::new(RichText::new(format!("{icon} {title}")).strong())
        .default_open(true)
        .show(ui, add_body);

    ui.separator();
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

    fn show_inspector_ui<'s>(
        &mut self,
        _: &'s Scene,
        _: EntityRef<'s>,
        _: &mut egui::Ui,
        _: &Resources,
    ) {
    }
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

    fn show_inspector_ui(
        &mut self,
        scene: &Scene,
        e: EntityRef<'_>,
        ui: &mut egui::Ui,
        resources: &Resources,
    ) {
        let mut rotation_euler: Vec3 = self.rotation.to_euler(glam::EulerRot::XYZ).into();
        rotation_euler.x = rotation_euler.x.to_degrees();
        rotation_euler.y = rotation_euler.y.to_degrees();
        rotation_euler.z = rotation_euler.z.to_degrees();

        let mut transform_changed = false;
        let mut rotation_changed = false;
        egui::Grid::new("transform_input_grid")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                if !self.flags.contains(TransformFlags::IGNORE_TRANSLATION) {
                    transform_changed |= input_float3!(
                        ui,
                        format!("{ICON_AXIS_ARROW} Translation"),
                        &mut self.translation
                    )
                    .inner;

                    ui.horizontal(|ui| {
                        let camera = resources.get::<Camera>();
                        if ui
                            .button(ICON_CAMERA_CONTROL.to_string())
                            .on_hover_text("Set position to camera")
                            .clicked()
                        {
                            self.translation = camera.position_target();
                            transform_changed |= true;
                        }

                        // TODO(cohae): Re-enable this when the renderer is back
                        // if let Some(renderer) = resources.get::<RendererShared>() {
                        //     let (d, pos) = renderer
                        //         .read()
                        //         .gbuffer
                        //         .depth_buffer_distance_pos_center(&camera);
                        //     if ui
                        //         .add_enabled(
                        //             d.is_finite(),
                        //             Button::new(if d.is_finite() {
                        //                 ICON_EYE_ARROW_RIGHT_OUTLINE.to_string()
                        //             } else {
                        //                 ICON_EYE_OFF_OUTLINE.to_string()
                        //             }),
                        //         )
                        //         .on_hover_text("Set position to gaze")
                        //         .clicked()
                        //     {
                        //         self.translation = pos;
                        //     }
                        //     ui.label(prettify_distance(d));
                        // }
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
                    transform_changed |= rotation_changed;
                    ui.end_row();
                }
                if !self.flags.contains(TransformFlags::IGNORE_SCALE) {
                    if self.flags.contains(TransformFlags::SCALE_IS_RADIUS) {
                        ui.label(format!("{ICON_RADIUS_OUTLINE} Radius"));
                        transform_changed |= egui::DragValue::new(&mut self.scale.x)
                            .speed(0.1)
                            .clamp_range(0f32..=f32::INFINITY)
                            .min_decimals(2)
                            .max_decimals(2)
                            .ui(ui)
                            .changed();

                        let camera = resources.get::<Camera>();
                        if ui
                            .button(ICON_RADIUS_OUTLINE.to_string())
                            .on_hover_text("Set radius to camera")
                            .clicked()
                        {
                            self.scale = Vec3::splat(
                                (self.translation - camera.position()).length().max(0.1),
                            );
                            transform_changed |= true;
                        }
                    } else {
                        transform_changed |=
                            input_float3!(ui, format!("{ICON_RESIZE} Scale"), &mut self.scale)
                                .inner;
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

        if transform_changed {
            if let Some((_static_instances, parent)) = e.query::<(&StaticInstance, &Parent)>().get()
            {
                if let Ok(mut static_instances) = scene.get::<&mut StaticInstances>(parent.0) {
                    static_instances.mark_dirty();
                }
            }

            if let Some(mut dynamic) = e.get::<&mut DynamicModelComponent>() {
                dynamic.mark_dirty();
            }
        }
    }
}

impl ComponentPanel for DynamicModelComponent {
    fn inspector_name() -> &'static str {
        "Dynamic Model"
    }

    fn inspector_icon() -> char {
        ICON_CUBE_OUTLINE
    }

    fn has_inspector_ui() -> bool {
        true
    }

    fn show_inspector_ui(&mut self, _: &Scene, _: EntityRef<'_>, ui: &mut egui::Ui, _: &Resources) {
        ui.horizontal(|ui| {
            ui.strong("Hash:");
            ui.label(self.model.hash.to_string());
        });
        ui.separator();

        let mesh_count = self.model.mesh_count();
        if mesh_count > 1 {
            egui::ComboBox::from_label("Mesh").show_index(
                ui,
                &mut self.model.selected_mesh,
                mesh_count,
                |i| format!("Mesh {i}"),
            );
        }

        let variant_count = self.model.variant_count();
        if variant_count > 1 {
            ui.style_mut().spacing.slider_width = 200.0;
            egui::Slider::new(&mut self.model.selected_variant, 0..=(variant_count - 1))
                .text("Material Variant")
                .ui(ui);
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

    fn show_inspector_ui(&mut self, _: &Scene, _: EntityRef<'_>, ui: &mut egui::Ui, _: &Resources) {
        ui.label(format!("World ID: 0x{:016X}", self.0));
    }
}
