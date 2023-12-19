use egui::RichText;
use glam::{Quat, Vec3};
use hecs::{Entity, EntityRef};

use crate::icons::{
    ICON_AXIS_ARROW, ICON_CUBE_OUTLINE, ICON_EYE, ICON_IDENTIFIER, ICON_MAP_MARKER,
};

use super::{
    components::{EntityWorldId, Label},
    transform::{OriginalTransform, Transform},
    Scene,
};

pub fn show_inspector_panel(ui: &mut egui::Ui, scene: &Scene, ent: Entity) {
    let Ok(e) = scene.entity(ent) else {
        return;
    };

    let title = if let Some(l) = e.get::<&Label>() {
        l.0.clone()
    } else {
        format!("ent_{}", ent.id())
    };

    ui.horizontal(|ui| {
        ui.add_enabled_ui(false, |ui| {
            ui.button(RichText::new(ICON_EYE).size(24.0).strong())
        });
        ui.label(RichText::new(title).size(24.0).strong());
    });
    ui.separator();

    show_inspector_components(ui, e);
}

fn show_inspector_components(ui: &mut egui::Ui, e: EntityRef<'_>) {
    if let Some(mut t) = e.get::<&mut Transform>() {
        inspector_component_frame(ui, "Transform", ICON_AXIS_ARROW, |ui| {
            t.show_inspector_ui(ui);
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
						component.show_inspector_ui(ui);
					});
				}
			)*
		};
	}

    component_views!(EntityWorldId);
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

trait ComponentPanel {
    fn inspector_name() -> &'static str;
    fn inspector_icon() -> char {
        ICON_CUBE_OUTLINE
    }
    fn has_inspector_ui() -> bool {
        false
    }
    fn show_inspector_ui(&mut self, _: &mut egui::Ui) {}
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

    fn show_inspector_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(format!("World ID: 0x{:016X}", self.0));
    }
}

impl ComponentPanel for Transform {
    fn inspector_name() -> &'static str {
        "Transform"
    }

    fn inspector_icon() -> char {
        ICON_MAP_MARKER
    }

    fn has_inspector_ui() -> bool {
        true
    }

    fn show_inspector_ui(&mut self, ui: &mut egui::Ui) {
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
                input_float3!(ui, "Translation", &mut self.translation);
                ui.end_row();
                rotation_changed = input_float3!(ui, "Rotation", &mut rotation_euler).inner;
                ui.end_row();
                input_float3!(ui, "Scale", &mut self.scale);
                ui.end_row();
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