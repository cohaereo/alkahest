use egui::{vec2, Color32, Rect, Sense, Vec2};

pub struct BigButton {
    icon: char,
    icon_color: Option<Color32>,
    label: String,
    subtext: Option<String>,
    full_width: bool,
}

impl BigButton {
    pub fn new(icon: char, label: impl AsRef<str>) -> Self {
        Self {
            icon,
            icon_color: None,
            label: label.as_ref().to_owned(),
            subtext: None,
            full_width: false,
        }
    }

    pub fn with_icon_color(mut self, color: Color32) -> Self {
        self.icon_color = Some(color);
        self
    }

    pub fn with_subtext(mut self, subtext: impl AsRef<str>) -> Self {
        self.subtext = Some(subtext.as_ref().to_owned());
        self
    }

    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let button_padding = ui.style().spacing.button_padding;

        let (ui_rect, response) = ui.allocate_exact_size(
            egui::vec2(
                if self.full_width {
                    ui.available_width() - button_padding.x / 2.
                } else {
                    320.0 - button_padding.x / 2.
                },
                72.0,
            ),
            Sense::click(),
        );

        let (frame_expansion, frame_rounding, frame_fill, frame_stroke) = {
            let visuals = ui.style().interact(&response);
            let expansion = Vec2::splat(visuals.expansion);
            (
                expansion,
                visuals.corner_radius,
                visuals.weak_bg_fill,
                visuals.bg_stroke,
            )
        };

        ui.painter().rect(
            ui_rect.expand2(frame_expansion),
            frame_rounding,
            frame_fill,
            frame_stroke,
            egui::StrokeKind::Middle,
        );

        let icon_rect = Rect::from_min_size(ui_rect.min, Vec2::splat(72.0));

        let painter = ui.painter_at(ui_rect);

        painter.text(
            icon_rect.translate(vec2(0.0, 5.0)).center(),
            egui::Align2::CENTER_CENTER,
            self.icon,
            egui::FontId::proportional(48.0),
            self.icon_color.unwrap_or(Color32::WHITE),
        );

        let subtext_offset = if self.subtext.is_some() {
            vec2(0.0, 6.0)
        } else {
            vec2(0.0, 0.0)
        };

        painter.text(
            icon_rect.right_center() - subtext_offset,
            egui::Align2::LEFT_CENTER,
            &self.label,
            egui::FontId::proportional(24.0),
            Color32::WHITE,
        );

        if let Some(subtext) = &self.subtext {
            painter.text(
                icon_rect.right_center() + vec2(0.0, 14.0) - subtext_offset,
                egui::Align2::LEFT_TOP,
                subtext,
                egui::FontId::proportional(12.0),
                Color32::GRAY,
            );
        }

        response
    }
}
