use alkahest_renderer::{
    ecs::{render::decorators::DecoratorRenderer, Scene},
    icons::ICON_TREE,
    renderer::RendererShared,
};
use bevy_ecs::prelude::EntityRef;
use egui::{Color32, Ui};

use crate::{gui::inspector::ComponentPanel, resources::AppResources};

impl ComponentPanel for DecoratorRenderer {
    fn inspector_name() -> &'static str {
        "Decorator"
    }

    fn inspector_icon() -> char {
        ICON_TREE
    }

    fn show_inspector_ui<'s>(
        &mut self,
        _: &'s mut Scene,
        _: EntityRef<'s>,
        ui: &mut Ui,
        resources: &AppResources,
    ) {
        let renderer = resources.get::<RendererShared>();
        renderer
            .immediate
            .cube_outline_aabb(&self.data.bounds, Color32::from_rgb(80, 210, 80));

        ui.horizontal(|ui| {
            ui.strong("Models:");
            ui.label(format!("{}", self.models.len()));
        });

        let mesh_count = self.models[0].0.mesh_count();
        if mesh_count > 1 {
            egui::ComboBox::from_label("Mesh").show_index(
                ui,
                &mut self.models[0].0.selected_mesh,
                mesh_count,
                |i| format!("Mesh {i}"),
            );
        }
    }
}
