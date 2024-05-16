use alkahest_data::map::{SLight, SShadowingLight};
use alkahest_renderer::{
    ecs::{light::LightRenderer, map::CubemapVolume, transform::Transform, Scene},
    icons::ICON_LIGHTBULB_ON,
    renderer::RendererShared,
    util::color::Color,
};
use destiny_pkg::TagHash;
use egui::{Color32, RichText, Ui};
use hecs::EntityRef;

use crate::{gui::inspector::ComponentPanel, resources::Resources};

impl ComponentPanel for LightRenderer {
    fn inspector_name() -> &'static str {
        "Light Renderer"
    }

    fn inspector_icon() -> char {
        ICON_LIGHTBULB_ON
    }

    fn has_inspector_ui() -> bool {
        true
    }

    fn show_inspector_ui<'s>(
        &mut self,
        _: &'s Scene,
        e: EntityRef<'s>,
        ui: &mut Ui,
        resources: &Resources,
        _: TagHash,
    ) {
        let renderer = resources.get::<RendererShared>();
        if !e.has::<SLight>() && !e.has::<SShadowingLight>() {
            ui.label(
                RichText::new("⚠ This light renderer is missing a (shadowing)light component")
                    .strong()
                    .color(Color32::RED),
            );
            return;
        }

        let is_shadowing = e.has::<SShadowingLight>();
        ui.horizontal(|ui| {
            ui.strong("Type:");
            ui.label(if is_shadowing {
                "Shadowing"
            } else {
                "Non-shadowing"
            });
        });
        ui.label(&self.debug_label);

        if let Some(shadowing) = e.get::<&SShadowingLight>() {
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

        if let Some(transform) = e.get::<&Transform>() {
            renderer.immediate.cube_outline(
                transform.local_to_world() * self.projection_matrix,
                Color::from_rgb(1.0, 1.0, 0.0),
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

    fn has_inspector_ui() -> bool {
        true
    }

    fn show_inspector_ui<'s>(
        &mut self,
        _: &'s Scene,
        e: EntityRef<'s>,
        _: &mut Ui,
        resources: &Resources,
        _: TagHash,
    ) {
        let renderer = resources.get::<RendererShared>();
        let transform = e.get::<&Transform>().expect("Volume missing Transform");
        renderer.immediate.cube_outline(
            Transform {
                scale: self.extents,
                ..*transform
            },
            Color::GREEN,
        );
    }
}
