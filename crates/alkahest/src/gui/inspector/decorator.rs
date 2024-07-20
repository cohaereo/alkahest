use alkahest_renderer::{
    ecs::{render::decorators::DecoratorRenderer, Scene},
    icons::ICON_TREE,
    renderer::RendererShared,
};
use egui::{Color32, Ui};
use hecs::EntityRef;

use crate::{gui::inspector::ComponentPanel, resources::Resources};

impl ComponentPanel for DecoratorRenderer {
    fn inspector_name() -> &'static str {
        "Decorator"
    }

    fn inspector_icon() -> char {
        ICON_TREE
    }

    fn has_inspector_ui() -> bool {
        true
    }

    fn show_inspector_ui<'s>(
        &mut self,
        _: &'s Scene,
        _: EntityRef<'s>,
        ui: &mut Ui,
        resources: &Resources,
    ) {
        let renderer = resources.get::<RendererShared>();
        renderer
            .immediate
            .cube_outline_aabb(&self.data.bounds, Color32::from_rgb(80, 210, 80));

        ui.horizontal(|ui| {
            ui.strong("Models:");
            ui.label(format!("{}", self.models.len()));
        });

        // let mesh_count = self.models[0].0.mesh_count();
        // if mesh_count > 1 {
        //     egui::ComboBox::from_label("Mesh").show_index(
        //         ui,
        //         &mut self.models[0].0.selected_mesh,
        //         mesh_count,
        //         |i| format!("Mesh {i}"),
        //     );
        // }
    }
}
