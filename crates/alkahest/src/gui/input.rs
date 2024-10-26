use egui::Response;

pub struct Vec4Input<'a> {
    pub value: &'a mut glam::Vec4,
}

impl<'a> Vec4Input<'a> {
    pub fn new(value: &'a mut glam::Vec4) -> Self {
        Self { value }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> Response {
        const DRAG_SPEED: f64 = 0.0025;
        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut self.value.x)
                    .prefix("x: ")
                    .speed(DRAG_SPEED),
            );
            ui.add(
                egui::DragValue::new(&mut self.value.y)
                    .prefix("y: ")
                    .speed(DRAG_SPEED),
            );
            ui.add(
                egui::DragValue::new(&mut self.value.z)
                    .prefix("z: ")
                    .speed(DRAG_SPEED),
            );
            ui.add(
                egui::DragValue::new(&mut self.value.w)
                    .prefix("w: ")
                    .speed(DRAG_SPEED),
            );
        })
        .response
    }
}

pub struct Vec3Input<'a> {
    pub value: &'a mut glam::Vec3,
}

impl<'a> Vec3Input<'a> {
    pub fn new(value: &'a mut glam::Vec3) -> Self {
        Self { value }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> Response {
        const DRAG_SPEED: f64 = 0.01;
        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut self.value.x)
                    .prefix("x: ")
                    .speed(DRAG_SPEED),
            );
            ui.add(
                egui::DragValue::new(&mut self.value.y)
                    .prefix("y: ")
                    .speed(DRAG_SPEED),
            );
            ui.add(
                egui::DragValue::new(&mut self.value.z)
                    .prefix("z: ")
                    .speed(DRAG_SPEED),
            );
        })
        .response
    }
}
