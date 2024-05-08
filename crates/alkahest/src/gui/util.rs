use egui::{vec2, Align2, Color32, FontId, Pos2, Rect};

pub trait PainterExt {
    fn text_with_shadow(
        &self,
        pos: Pos2,
        anchor: Align2,
        text: impl ToString,
        font_id: FontId,
        text_color: Color32,
    ) -> Rect;
}

impl PainterExt for egui::Painter {
    fn text_with_shadow(
        &self,
        pos: Pos2,
        anchor: Align2,
        text: impl ToString,
        font_id: FontId,
        text_color: Color32,
    ) -> Rect {
        self.text(
            pos + vec2(1.0, 1.0),
            anchor,
            text.to_string(),
            font_id.clone(),
            Color32::BLACK,
        );

        self.text(pos, anchor, text.to_string(), font_id, text_color)
    }
}
