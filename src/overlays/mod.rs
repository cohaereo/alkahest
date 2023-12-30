use egui::Response;

pub mod camera_settings;
pub mod console;
pub mod fps_display;
pub mod gui;
pub mod inspector;
pub mod load_indicator;
pub mod material_viewer;
pub mod outliner;
pub mod render_settings;
pub mod resource_nametags;
pub mod tag_dump;
pub mod texture_viewer;

pub mod chip;

pub trait UiExt {
    fn chip(&mut self, label: impl AsRef<str>) -> Response;

    fn chip_with_color(&mut self, label: impl AsRef<str>, color: egui::Color32) -> Response;
}

impl UiExt for egui::Ui {
    fn chip(&mut self, label: impl AsRef<str>) -> Response {
        chip::Chip::from_str(&label).ui(self)
    }

    fn chip_with_color(&mut self, label: impl AsRef<str>, color: egui::Color32) -> Response {
        chip::Chip::from_str(&label).with_color(color).ui(self)
    }
}
