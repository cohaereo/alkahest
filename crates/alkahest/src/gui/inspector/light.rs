use alkahest_data::map::{SLight, SLightCollection, SShadowingLight};
use alkahest_renderer::{
    ecs::{
        hierarchy::Children, map::CubemapVolume, render::light::LightRenderer,
        transform::Transform, Scene,
    },
    icons::{ICON_LIGHTBULB_GROUP, ICON_LIGHTBULB_ON},
    renderer::RendererShared,
    util::color::Color,
};
use bevy_ecs::{prelude::EntityRef, system::Commands};
use egui::{Color32, RichText, Ui};

use crate::{gui::inspector::ComponentPanel, resources::AppResources};

impl ComponentPanel for SLightCollection {
    fn inspector_name() -> &'static str {
        "Light Collection"
    }

    fn inspector_icon() -> char {
        ICON_LIGHTBULB_GROUP
    }

    fn show_inspector_ui<'s>(
        &mut self,
        scene: &'s mut Scene,
        _: &mut Commands<'_, '_>,
        e: EntityRef<'s>,
        _ui: &mut Ui,
        resources: &AppResources,
    ) {
        let renderer = resources.get::<RendererShared>();
        let Some(children) = e.get::<Children>() else {
            return;
        };

        for child in &children.0 {
            if let Ok((light, transform)) = scene
                .query::<(&LightRenderer, &Transform)>()
                .get(scene, *child)
            {
                renderer.immediate.cube_outline(
                    transform.local_to_world() * light.projection_matrix,
                    Color::from_rgb(1.0, 1.0, 0.0),
                );

                renderer.immediate.sphere(
                    transform.translation,
                    0.04,
                    Color::from_rgba_premultiplied(1.0, 1.0, 0.0, 0.9),
                )
            }
        }
    }
}

impl ComponentPanel for LightRenderer {
    fn inspector_name() -> &'static str {
        "Light Renderer"
    }

    fn inspector_icon() -> char {
        ICON_LIGHTBULB_ON
    }

    fn show_inspector_ui<'s>(
        &mut self,
        _: &'s mut Scene,
        _: &mut Commands<'_, '_>,
        e: EntityRef<'s>,
        ui: &mut Ui,
        resources: &AppResources,
    ) {
        let renderer = resources.get::<RendererShared>();
        if !e.contains::<SLight>() && !e.contains::<SShadowingLight>() {
            ui.label(
                RichText::new("⚠ This light renderer is missing a (shadowing)light component")
                    .strong()
                    .color(Color32::RED),
            );
            return;
        }

        let is_shadowing = e.contains::<SShadowingLight>();
        ui.horizontal(|ui| {
            ui.strong("Type:");
            ui.label(if is_shadowing {
                "Shadowing"
            } else {
                "Non-shadowing"
            });
        });
        ui.label(&self.debug_label);
        ui.collapsing("Debug Info", |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            ui.label(&self.debug_info);
        });

        if let Some(shadowing) = e.get::<SShadowingLight>() {
            ui.separator();
            ui.horizontal(|ui| {
                ui.strong("FoV:");
                ui.label(format!("{:.1}", (shadowing.half_fov * 2.).to_degrees()));
            });
            ui.horizontal(|ui| {
                ui.strong("Far Plane:");
                ui.label(format!("{:.1}", shadowing.far_plane));
            });
        }

        if let Some(transform) = e.get::<Transform>() {
            renderer.immediate.cube_outline(
                transform.local_to_world() * self.projection_matrix,
                Color::from_rgb(1.0, 1.0, 0.0),
            );

            renderer.immediate.sphere(
                transform.translation,
                0.04,
                Color::from_rgba_premultiplied(1.0, 1.0, 0.0, 0.9),
            )
        } else {
            ui.label(
                RichText::new("⚠ This light renderer is missing a transform component")
                    .strong()
                    .color(Color32::RED),
            );
        }
    }
}

impl ComponentPanel for CubemapVolume {
    fn inspector_name() -> &'static str {
        "Cubemap Volume"
    }

    fn inspector_icon() -> char {
        ICON_LIGHTBULB_ON
    }

    fn show_inspector_ui<'s>(
        &mut self,
        _: &'s mut Scene,
        _: &mut Commands<'_, '_>,
        e: EntityRef<'s>,
        _: &mut Ui,
        resources: &AppResources,
    ) {
        let renderer = resources.get::<RendererShared>();
        let transform = e.get::<Transform>().expect("Volume missing Transform");
        renderer.immediate.cube_outline(
            Transform {
                scale: self.extents,
                ..*transform
            },
            Color::GREEN,
        );
    }
}
