use egui::{epaint, vec2, Color32, Rect, Response, RichText};

use crate::util::text::{name_to_color, text_color_for_background};

// Mmm, chip.
pub struct Chip {
    label: String,
    background: Color32,
    size: f32,
}

impl Chip {
    pub fn from_str(label: &impl AsRef<str>) -> Self {
        Self {
            label: label.as_ref().to_owned(),
            background: name_to_color(label.as_ref()),
            size: 14.0,
        }
    }

    pub fn with_color(self, background: Color32) -> Self {
        Self { background, ..self }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> Response {
        let size_ratio = self.size / 14.0;

        let wrap_og = ui.style().wrap;
        ui.style_mut().wrap = Some(false);

        let text_color = text_color_for_background(self.background);
        let label = egui::Label::new(
            RichText::from(&self.label)
                .color(text_color)
                .strong()
                .size(12.0),
        );
        let (pos, text_galley, response) = label.layout_in_ui(ui);
        let rect = Rect::from_min_size(pos, text_galley.size());

        if ui.is_rect_visible(rect) {
            ui.painter().rect_filled(
                rect.expand2(vec2(2.5, 1.25) * size_ratio),
                4.0,
                self.background,
            );
            ui.painter().add(epaint::TextShape {
                pos,
                galley: text_galley,
                override_text_color: Some(text_color),
                fallback_color: Color32::TRANSPARENT,
                underline: Default::default(),
                angle: 0.0,
            });
        }

        ui.style_mut().wrap = wrap_og;

        response
    }
}
