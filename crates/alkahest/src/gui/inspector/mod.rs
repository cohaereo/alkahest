mod decorator;
mod light;
mod util;

use alkahest_data::map::{SLightCollection, SRespawnPoint};
use alkahest_renderer::{
    camera::Camera,
    ecs::{
        common::{Global, Label, Mutable},
        hierarchy::{Children, Parent},
        map::{CubemapVolume, NodeMetadata},
        render::{
            decorators::DecoratorRenderer, dynamic_geometry::DynamicModelComponent,
            light::LightRenderer,
        },
        resources::SelectedEntity,
        tags::{insert_tag, remove_tag, EntityTag, Tags},
        transform::{OriginalTransform, Transform, TransformFlags},
        utility::{Beacon, Route, RouteNode, Ruler, Sphere},
        visibility::{Visibility, VisibilityHelper},
        Scene,
    },
    icons::{
        ICON_ACCOUNT_CONVERT, ICON_EYE_ARROW_RIGHT_OUTLINE, ICON_HUMAN_MALE,
        ICON_HUMAN_MALE_FEMALE_CHILD, ICON_POKEBALL,
    },
    renderer::RendererShared,
    shader::shader_ball::ShaderBallComponent,
    util::{black_magic::EntityRefDarkMagic, Hocus},
};
use bevy_ecs::{entity::Entity, prelude::EntityRef, system::Commands};
use egui::{Align2, Color32, FontId, Key, RichText, Ui, Widget};
use glam::{Quat, Vec3};
use winit::window::Window;

use crate::{
    gui::{
        chip::EcsTagsExt,
        context::{GuiCtx, GuiView, ViewResult},
        hotkeys::{SHORTCUT_DELETE, SHORTCUT_HIDE},
        icons::{
            ICON_AXIS_ARROW, ICON_CAMERA_CONTROL, ICON_CUBE_OUTLINE, ICON_DELETE, ICON_EYE,
            ICON_EYE_OFF, ICON_RADIUS_OUTLINE, ICON_RESIZE, ICON_ROTATE_ORBIT, ICON_TAG,
        },
    },
    input_float3,
    maplist::MapList,
    resources::AppResources,
};

pub struct InspectorPanel;

impl GuiView for InspectorPanel {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &Window,
        resources: &AppResources,
        _gui: &GuiCtx<'_>,
    ) -> Option<ViewResult> {
        let mut maps = resources.get_mut::<MapList>();

        if let Some(map) = maps.current_map_mut() {
            egui::Window::new("Inspector").show(ctx, |ui| {
                let selected = resources.get::<SelectedEntity>().selected();
                if let Some(ent) = selected {
                    show_inspector_panel(
                        ui,
                        &mut map.pocus().scene,
                        map.commands(),
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

pub fn show_inspector_panel(
    ui: &mut egui::Ui,
    scene: &mut Scene,
    mut cmd: Commands<'_, '_>,
    ent: Entity,
    resources: &AppResources,
) {
    let Some(e) = scene.get_entity(ent) else {
        return;
    };

    ui.horizontal(|ui| {
        let visible = e.get::<Visibility>().is_visible(0);

        if e.contains::<Mutable>()
            && (ui
                .button(RichText::new(ICON_DELETE).size(24.0).strong())
                .clicked()
                || ui.input_mut(|i| i.consume_shortcut(&SHORTCUT_DELETE)))
        {
            // Remove all children
            if let Some(children) = e.get::<Children>() {
                for child_ent in &children.0 {
                    cmd.entity(*child_ent).despawn();
                }
            }
            // Remove from parent
            if let Some(parent) = e.get::<Parent>() {
                if let Some(mut children) = scene.entity(parent.0).get_mut::<Children>() {
                    children.0.retain(|child| *child != e.id());
                }
                resources.get_mut::<SelectedEntity>().select(parent.0);
            }
            cmd.entity(ent).despawn();
        }

        if e.contains::<RouteNode>() {
            ui.label(
                RichText::new(if visible { ICON_EYE } else { ICON_EYE_OFF })
                    .size(24.0)
                    .strong(),
            );
        } else if ui
            .button(
                RichText::new(if visible { ICON_EYE } else { ICON_EYE_OFF })
                    .size(24.0)
                    .strong(),
            )
            .clicked()
            || ui.input_mut(|i| i.consume_shortcut(&SHORTCUT_HIDE))
        {
            if visible {
                cmd.entity(ent).insert((Visibility::Hidden,));
            } else {
                cmd.entity(ent).insert((Visibility::Visible,));
            }
        }

        let title = if let Some(label) = e.get::<Label>() {
            format!("{label} (id {})", ent)
        } else {
            format!("Entity {}", ent)
        };
        ui.horizontal(|ui| {
            if e.contains::<Mutable>() {
                let some_label = e.get_mut::<Label>();
                if some_label.as_ref().is_some_and(|l| !l.default) {
                    if let Some(mut label) = some_label {
                        egui::TextEdit::singleline(&mut label.label)
                            .font(FontId::proportional(22.0))
                            .ui(ui);
                    }
                } else {
                    ui.label(RichText::new(title).size(24.0).strong());
                    if ui
                        .button(RichText::new(ICON_TAG.to_string()).size(24.0).strong())
                        .on_hover_text("Add label")
                        .clicked()
                    {
                        if let Some(mut label) = some_label {
                            label.default = false;
                        } else {
                            cmd.entity(ent)
                                .insert((Label::from(format!("Entity {}", e.id())),));
                        }
                    }
                }
            } else {
                ui.label(RichText::new(title).size(24.0).strong());
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Max), |ui| {
                if let Some(parent) = e.get::<Parent>() {
                    let title = if let Some(label) = scene.get::<Label>(parent.0) {
                        format!("{label} (id {})", e.id())
                    } else {
                        format!("Entity {}", e.id())
                    };

                    if ui
                        .button(RichText::new(ICON_HUMAN_MALE.to_string()).size(20.0))
                        .on_hover_text(format!("Parent: {title}"))
                        .clicked()
                    {
                        resources.get_mut::<SelectedEntity>().select(parent.0);
                    }
                }
                if let Some(children) = e.get::<Children>() {
                    ui.menu_button(
                        RichText::new(ICON_HUMAN_MALE_FEMALE_CHILD.to_string()).size(20.0),
                        |ui| {
                            for c in children.iter() {
                                let title = if let Some(label) = scene.get::<Label>(*c) {
                                    format!("{label} (id {})", *c)
                                } else {
                                    format!("Entity {}", *c)
                                };

                                if ui.selectable_label(false, title).clicked() {
                                    resources.get_mut::<SelectedEntity>().select(*c);
                                }
                            }
                        },
                    )
                    .response
                    .on_hover_text("Children");
                }
            });
        });
    });
    ui.separator();

    if let Some(tags) = e.get::<Tags>() {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Tags: ").color(Color32::WHITE).strong());
            tags.ui_chips(ui);
        });
        ui.separator();
    }

    let mut global = e.contains::<Global>();
    let mut global_changed = false;
    if e.contains::<Mutable>() && !e.contains::<Route>() && !e.contains::<RouteNode>() {
        if ui.checkbox(&mut global, "Show in all Maps").changed() {
            global_changed = true;
            if global {
                cmd.entity(ent).insert((Global,));
            } else {
                cmd.entity(ent).remove::<Global>();
            }
        };
        ui.separator();
    }
    show_inspector_components(ui, scene.pocus(), &mut cmd, e, resources);

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
    scene: &mut Scene,
    cmd: &mut Commands<'_, '_>,
    e: EntityRef<'_>,
    resources: &AppResources,
) {
    if let Some(mut t) = e.get_mut::<Transform>() {
        inspector_component_frame(ui, "Transform", ICON_AXIS_ARROW, |ui| {
            t.show_inspector_ui(scene, cmd, e, ui, resources);
        });
    }

    macro_rules! component_views {
		($($component:ty),+) => {
			$(
				if let Some(mut component) = e.get_mut::<$component>() {
					inspector_component_frame(ui, <$component>::inspector_name(), <$component>::inspector_icon(), |ui| {
						component.show_inspector_ui(scene, cmd, e, ui, resources);
					});
				}
			)*
		};
	}

    component_views!(
        // EntityWorldId,
        Ruler,
        Sphere,
        Beacon,
        Route,
        RouteNode,
        DynamicModelComponent,
        LightRenderer,
        SLightCollection,
        CubemapVolume,
        ShaderBallComponent,
        DecoratorRenderer,
        SRespawnPoint,
        NodeMetadata
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

    fn show_inspector_ui<'s>(
        &mut self,
        _: &'s mut Scene,
        _: &mut Commands<'_, '_>,
        _: EntityRef<'s>,
        _: &mut egui::Ui,
        _: &AppResources,
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

    fn show_inspector_ui(
        &mut self,
        _scene: &mut Scene,
        _: &mut Commands<'_, '_>,
        e: EntityRef<'_>,
        ui: &mut egui::Ui,
        resources: &AppResources,
    ) {
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
                        }

                        // let (d, pos) = resources
                        //     .get::<RendererShared>()
                        //     .data
                        //     .lock()
                        //     .gbuffers
                        //     .depth_buffer_distance_pos_center(&camera);
                        // if ui
                        //     .add_enabled(
                        //         d.is_finite(),
                        //         Button::new(if d.is_finite() {
                        //             ICON_EYE_ARROW_RIGHT_OUTLINE.to_string()
                        //         } else {
                        //             ICON_EYE_OFF_OUTLINE.to_string()
                        //         }),
                        //     )
                        //     .on_hover_text("Set position to gaze")
                        //     .clicked()
                        // {
                        //     self.translation = pos;
                        // }
                        // ui.label(prettify_distance(d));

                        if ui
                            .button(ICON_EYE_ARROW_RIGHT_OUTLINE.to_string())
                            .on_hover_text("Set position to gaze")
                            .clicked()
                        {
                            let (distance, pos) = resources
                                .get::<RendererShared>()
                                .data
                                .lock()
                                .gbuffers
                                .depth_buffer_distance_pos_center(&camera);
                            if distance.is_finite() {
                                self.translation = pos;
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
                        egui::DragValue::new(&mut self.scale.x)
                            .speed(0.1)
                            .range(0f32..=f32::INFINITY)
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
                        }
                    } else {
                        input_float3!(ui, format!("{ICON_RESIZE} Scale"), &mut self.scale).inner;
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

        if let Some(ot) = e.get::<OriginalTransform>() {
            // Has the entity moved from it's original position?
            let has_moved = *self != ot.0;
            ui.add_enabled_ui(has_moved, |ui: &mut egui::Ui| {
                if ui
                    .button("Reset to original")
                    .on_hover_text(
                        "This object has an original transform defined.\nClicking this button \
                         will reset the current transform back  to the original",
                    )
                    .clicked()
                {
                    *self = ot.0;
                }
            });
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

    fn show_inspector_ui(
        &mut self,
        _: &mut Scene,
        _: &mut Commands<'_, '_>,
        _: EntityRef<'_>,
        ui: &mut egui::Ui,
        _: &AppResources,
    ) {
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

        let identifier_count = self.model.identifier_count();
        if identifier_count > 1 {
            egui::ComboBox::from_label("Identifier")
                .selected_text(if self.identifier == u16::MAX {
                    "All".to_string()
                } else {
                    format!("ID {}", self.identifier)
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.identifier, u16::MAX, "All");
                    for i in 0..identifier_count {
                        ui.selectable_value(&mut self.identifier, i as u16, format!("ID {i}"));
                    }

                    if ui.input(|i| i.key_pressed(Key::ArrowUp)) {
                        if self.identifier == u16::MAX {
                            self.identifier = identifier_count as u16 - 1;
                        } else {
                            self.identifier = self.identifier.wrapping_sub(1);
                        }
                    }

                    if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
                        if self.identifier == u16::MAX {
                            self.identifier = 0;
                        } else {
                            self.identifier = self.identifier.wrapping_add(1);
                            if self.identifier >= identifier_count as u16 {
                                self.identifier = u16::MAX;
                            }
                        }
                    }
                });
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

impl ComponentPanel for ShaderBallComponent {
    fn inspector_name() -> &'static str {
        "Shader Ball"
    }

    fn inspector_icon() -> char {
        ICON_POKEBALL
    }

    fn show_inspector_ui(
        &mut self,
        _: &mut Scene,
        _: &mut Commands<'_, '_>,
        _: EntityRef<'_>,
        ui: &mut egui::Ui,
        _: &AppResources,
    ) {
        ui.horizontal(|ui| {
            ui.strong("Color:");
            ui.color_edit_button_rgb(self.color.as_mut());
        });
        ui.horizontal(|ui| {
            ui.strong("Iridescence:");
            egui::Slider::new(&mut self.iridescence, 0..=126).ui(ui);
        });
        ui.horizontal(|ui| {
            ui.strong("Emissive:");
            egui::Slider::new(&mut self.emission, 0.0..=1.0).ui(ui);
        });
        ui.horizontal(|ui| {
            ui.strong("Metalness:");
            egui::Slider::new(&mut self.metalness, 0.0..=1.0).ui(ui);
        });
        ui.horizontal(|ui| {
            ui.strong("Fuzz/Smoothness:");
            egui::Slider::new(&mut self.smoothness, -1.0..=1.0).ui(ui);
        });
        ui.horizontal(|ui| {
            ui.strong("Transmission:");
            egui::Slider::new(&mut self.transmission, 0.0..=1.0).ui(ui);
        });
    }
}

// impl ComponentPanel for EntityWorldId {
//     fn inspector_name() -> &'static str {
//         "World ID"
//     }
//
//     fn inspector_icon() -> char {
//         ICON_IDENTIFIER
//     }
//
//     fn show_inspector_ui(&mut self, _: &Scene, _: EntityRef<'_>, ui: &mut egui::Ui, _: &Resources) {
//         ui.label(format!("World ID: 0x{:016X}", self.0));
//     }
// }

impl ComponentPanel for NodeMetadata {
    fn inspector_name() -> &'static str {
        "Node Metadata"
    }

    fn inspector_icon() -> char {
        ICON_TAG
    }

    fn show_inspector_ui<'s>(
        &mut self,
        _: &'s mut Scene,
        _: &mut Commands<'_, '_>,
        _: EntityRef<'s>,
        ui: &mut Ui,
        _: &AppResources,
    ) {
        ui.horizontal(|ui| {
            ui.strong("Entity:");
            ui.label(self.entity_tag.to_string());
        });

        ui.horizontal(|ui| {
            ui.strong("World ID:");
            ui.label(format!("0x{:016X}", self.world_id));
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.strong("Source Table:");
            ui.label(self.source_table.to_string());
        });

        ui.horizontal(|ui| {
            ui.strong("Resource Type:");
            ui.label(format!(
                "{:08X} (offset 0x{:X})",
                self.resource_type, self.source_table_resource_offset
            ));
        });
    }
}

impl ComponentPanel for SRespawnPoint {
    fn inspector_name() -> &'static str {
        "Respawn Point"
    }

    fn inspector_icon() -> char {
        ICON_ACCOUNT_CONVERT
    }

    fn show_inspector_ui<'s>(
        &mut self,
        _: &'s mut Scene,
        _: &mut Commands<'_, '_>,
        _: EntityRef<'s>,
        ui: &mut Ui,
        _: &AppResources,
    ) {
        ui.strong("Respawn point data");
        ui.indent("Respawn point data", |ui| {
            ui.monospace(format!("{self:#X?}"));
        });
    }
}
