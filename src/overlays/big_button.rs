use egui::{vec2, Color32, Rect, Sense, Vec2};

pub struct BigButton {
    icon: char,
    icon_color: Option<Color32>,
    label: String,
    // TODO(cohae): Subtext would be cool, but it's kind of difficult to do this in immediate mode UI
    // subtext: Option<String>,
}

impl BigButton {
    pub fn new(icon: char, label: impl AsRef<str>) -> Self {
        Self {
            icon,
            icon_color: None,
            label: label.as_ref().to_owned(),
            // subtext: None,
        }
    }

    pub fn with_icon_color(mut self, color: Color32) -> Self {
        self.icon_color = Some(color);
        self
    }

    // pub fn with_subtext(mut self, subtext: impl AsRef<str>) -> Self {
    //     self.subtext = Some(subtext.as_ref().to_owned());
    //     self
    // }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let (ui_rect, response) = ui.allocate_exact_size(egui::vec2(300.0, 72.0), Sense::click());

        let (frame_expansion, frame_rounding, frame_fill, frame_stroke) = {
            let visuals = ui.style().interact(&response);
            let expansion = Vec2::splat(visuals.expansion);
            (
                expansion,
                visuals.rounding,
                visuals.weak_bg_fill,
                visuals.bg_stroke,
            )
        };

        ui.painter().rect(
            ui_rect.expand2(frame_expansion),
            frame_rounding,
            frame_fill,
            frame_stroke,
        );

        let icon_rect = Rect::from_min_size(ui_rect.min, Vec2::splat(72.0));

        ui.painter().text(
            icon_rect.translate(vec2(0.0, 7.0)).center(),
            egui::Align2::CENTER_CENTER,
            self.icon,
            egui::FontId::proportional(48.0),
            self.icon_color.unwrap_or(Color32::WHITE),
        );

        ui.painter().text(
            icon_rect.right_center(),
            egui::Align2::LEFT_CENTER,
            &self.label,
            egui::FontId::proportional(24.0),
            Color32::WHITE,
        );

        response

        //     let mut response = egui::Response::new(egui::Id::new("big_button"));
        //     let rect = response.rect;

        //     let mut content = egui::Align2::centered(egui::Vec2::splat(32.0));
        //     content = content.add(egui::Label::new(self.icon).text_style(egui::TextStyle::Heading));
        //     content =
        //         content.add(egui::Label::new(self.label.as_str()).text_style(egui::TextStyle::Heading));

        //     if let Some(subtext) = &self.subtext {
        //         content =
        //             content.add(egui::Label::new(subtext.as_str()).text_style(egui::TextStyle::Small));
        //     }

        //     let mut content = egui::Widget::default().fill(egui::Color32::from_rgb(0, 0, 0));
        //     content = content.border(egui::Stroke::new(
        //         1.0,
        //         egui::Color32::from_rgb(255, 255, 255),
        //     ));
        //     content = content.centered();
        //     content = content.add(content);

        //     let mut content = egui::Widget::default().fill(egui::Color32::from_rgb(0, 0, 0));
        //     content = content.border(egui::Stroke::new(
        //         1.0,
        //         egui::Color32::from_rgb(255, 255, 255),
        //     ));
        //     content = content.centered();
        //     content = content.add(content);

        //     let mut content = egui::Widget::default().fill(egui::Color32::from_rgb(0, 0, 0));
        //     content = content.border(egui::Stroke::new(
        //         1.0,
        //         egui::Color32::from_rgb(255, 255, 255),
        //     ));
        //     content = content.centered();
        //     content = content.add(content);
    }
}
