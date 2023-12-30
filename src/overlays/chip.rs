use egui::{epaint, vec2, Color32, Rect, Response, RichText};

use crate::util::fnv1;

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
                galley: text_galley.galley,
                override_text_color: Some(text_color),
                underline: Default::default(),
                angle: 0.0,
            });
        }

        ui.style_mut().wrap = wrap_og;

        response
    }
}

pub fn text_color_for_background(background: Color32) -> Color32 {
    let r = background.r() as f32 / 255.;
    let g = background.g() as f32 / 255.;
    let b = background.b() as f32 / 255.;
    let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;

    if luma > 0.5 {
        Color32::BLACK
    } else {
        Color32::WHITE
    }
}

pub fn name_to_color(name: &str) -> Color32 {
    let hash = fnv1(name.as_bytes());
    let r = (hash & 0xFF) as u8;
    let g = ((hash >> 8) & 0xFF) as u8;
    let b = ((hash >> 16) & 0xFF) as u8;
    Color32::from_rgb(r, g, b)
}
