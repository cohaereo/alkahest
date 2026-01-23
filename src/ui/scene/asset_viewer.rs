use std::any::TypeId;

use alkahest_render::asset::texture::Texture;
use egui::{FontId, RichText, TextStyle, vec2};
use itertools::Itertools;

impl super::Scene {
    pub(super) fn show_texture_viewer(
        &mut self,
        ui: &mut egui::Ui,
        renderer: &mut egui_d3d11::D3D11Renderer,
    ) {
        let mut asset_handles = self
            .renderer
            .asset_manager
            .assets
            .lock()
            .iter()
            .filter_map(|(tag, (asset_type, handle))| {
                let is_texture = *asset_type == TypeId::of::<Texture>();
                is_texture.then_some((*tag, handle.clone()))
            })
            .collect_vec();

        ui.style_mut()
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(16.0));

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (tag, untyped_handle) in asset_handles.iter() {
                let handle = unsafe { untyped_handle.clone_as_typed_unchecked::<Texture>() };
                let Some(texture) = handle.get() else {
                    continue;
                };
                let texture_id = renderer.textures_mut().allocate_dx_temporary(
                    texture.view.clone(),
                    Some(egui::TextureFilter::Linear),
                    false,
                );

                let (width, height) = texture.resolution();
                let vertical_aspect_ratio = if width > height {
                    // Landscape
                    height as f32 / width as f32
                } else {
                    // Portrait
                    height as f32 / width as f32
                };

                ui.label(format!("Texture {tag}"));
                let (rect, _) = ui.allocate_exact_size(
                    vec2(128.0, 128.0 * vertical_aspect_ratio),
                    egui::Sense::hover(),
                );

                ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);
                ui.painter().image(
                    texture_id,
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );

                ui.separator();
            }
        });
    }
}
