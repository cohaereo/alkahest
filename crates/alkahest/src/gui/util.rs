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

#[macro_export]
macro_rules! input_float3 {
    ($ui:expr, $label:expr, $v:expr) => {{
        $ui.label($label);
        $ui.horizontal(|ui| {
            let c0 = ui
                .add(
                    egui::DragValue::new(&mut $v.x)
                        .speed(0.1)
                        .prefix("x: ")
                        .min_decimals(2)
                        .max_decimals(2),
                )
                .changed();
            let c1 = ui
                .add(
                    egui::DragValue::new(&mut $v.y)
                        .speed(0.1)
                        .prefix("y: ")
                        .min_decimals(2)
                        .max_decimals(2),
                )
                .changed();
            let c2 = ui
                .add(
                    egui::DragValue::new(&mut $v.z)
                        .speed(0.1)
                        .prefix("z: ")
                        .min_decimals(2)
                        .max_decimals(2),
                )
                .changed();

            c0 || c1 || c2
        })
    }};
}

#[macro_export]
macro_rules! input_float4 {
    ($ui:expr, $label:expr, $v:expr) => {{
        $ui.label($label);
        $ui.horizontal(|ui| {
            let c0 = ui
                .add(
                    egui::DragValue::new(&mut $v.x)
                        .speed(0.1)
                        .prefix("x: ")
                        .min_decimals(2)
                        .max_decimals(2),
                )
                .changed();
            let c1 = ui
                .add(
                    egui::DragValue::new(&mut $v.y)
                        .speed(0.1)
                        .prefix("y: ")
                        .min_decimals(2)
                        .max_decimals(2),
                )
                .changed();
            let c2 = ui
                .add(
                    egui::DragValue::new(&mut $v.z)
                        .speed(0.1)
                        .prefix("z: ")
                        .min_decimals(2)
                        .max_decimals(2),
                )
                .changed();
            let c3 = ui
                .add(
                    egui::DragValue::new(&mut $v.w)
                        .speed(0.1)
                        .prefix("w: ")
                        .min_decimals(2)
                        .max_decimals(2),
                )
                .changed();

            c0 || c1 || c2 || c3
        })
    }};
}
