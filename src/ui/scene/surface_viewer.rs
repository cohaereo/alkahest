use egui::{FontId, RichText, TextStyle, vec2};

impl super::Scene {
    pub(super) fn show_surface_viewer(
        &mut self,
        ui: &mut egui::Ui,
        renderer: &mut egui_d3d11::D3D11Renderer,
    ) {
        ui.style_mut()
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(16.0));

        let surfaces = self.view.surfaces().unwrap();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (_handle, surface) in surfaces.iter() {
                let texture_id = surface.srv(0).cloned().map(|srv| {
                    renderer
                        .textures_mut()
                        .allocate_dx_temporary(srv, None, false)
                });

                let (width, height) = surface.resolution();
                let vertical_aspect_ratio = if width > height {
                    // Landscape
                    height as f32 / width as f32
                } else {
                    // Portrait
                    height as f32 / width as f32
                };

                ui.label(surface.name());
                ui.weak(
                    RichText::new(format!(
                        "{}x{} ({:?}, {}, {:?})",
                        surface.resolution().0,
                        surface.resolution().1,
                        surface.desc().format,
                        surface.desc().scale,
                        surface.desc().size_relativity
                    ))
                    .size(12.0),
                );
                if let Some(tid) = texture_id {
                    let (rect, _) = ui.allocate_exact_size(
                        vec2(384.0, 384.0 * vertical_aspect_ratio),
                        egui::Sense::hover(),
                    );

                    ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);
                    ui.painter().image(
                        tid,
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                } else {
                    ui.label("No SRV available");
                }

                ui.separator();
            }
        });
    }
}
