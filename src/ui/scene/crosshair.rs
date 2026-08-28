use egui::{Color32, Pos2, Stroke, Ui};

pub fn draw_crosshair(ui: &mut Ui, center: Pos2) {
    let painter = ui.ctx().debug_painter();
    let width = 2.0;
    let size = 8.0;
    let stroke = Stroke::new(width, Color32::WHITE);

    painter.line_segment(
        [
            Pos2::new(center.x - size, center.y),
            Pos2::new(center.x + size, center.y),
        ],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(center.x, center.y - size),
            Pos2::new(center.x, center.y + size),
        ],
        stroke,
    );
}
