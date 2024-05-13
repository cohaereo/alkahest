use egui::Response;

pub mod activity_select;
mod configuration;
pub mod context;
mod fps_display;
pub mod hotkeys;
pub use alkahest_renderer::icons;
mod input;
pub mod inspector;
mod tfx;

// Custom widgets
pub mod big_button;
mod bottom_bar;
pub mod chip;
mod menu;
mod outliner;
mod util;

pub trait UiExt {
    fn chip(&mut self, label: impl AsRef<str>) -> Response;

    fn chip_with_color(&mut self, label: impl AsRef<str>, color: egui::Color32) -> Response;

    fn big_button(&mut self, icon: char, label: impl AsRef<str>) -> Response;

    fn big_button_with_icon_color(
        &mut self,
        icon: char,
        label: impl AsRef<str>,
        color: egui::Color32,
    ) -> Response;

    // fn big_button_with_subtext(
    //     &mut self,
    //     icon: char,
    //     label: impl AsRef<str>,
    //     subtext: impl AsRef<str>,
    // ) -> Response;

    fn vec4_input(&mut self, value: &mut glam::Vec4) -> Response;
    
    fn vec3_input(&mut self, value: &mut glam::Vec3) -> Response;
}

impl UiExt for egui::Ui {
    fn chip(&mut self, label: impl AsRef<str>) -> Response {
        chip::Chip::from_str(&label).ui(self)
    }

    fn chip_with_color(&mut self, label: impl AsRef<str>, color: egui::Color32) -> Response {
        chip::Chip::from_str(&label).with_color(color).ui(self)
    }

    fn big_button(&mut self, icon: char, label: impl AsRef<str>) -> Response {
        big_button::BigButton::new(icon, label).ui(self)
    }

    fn big_button_with_icon_color(
        &mut self,
        icon: char,
        label: impl AsRef<str>,
        color: egui::Color32,
    ) -> Response {
        big_button::BigButton::new(icon, label)
            .with_icon_color(color)
            .ui(self)
    }

    // fn big_button_with_subtext(
    //     &mut self,
    //     icon: char,
    //     label: impl AsRef<str>,
    //     subtext: impl AsRef<str>,
    // ) -> Response {
    //     big_button::BigButton::new(icon, label)
    //         .with_subtext(subtext)
    //         .ui(self)
    // }

    fn vec4_input(&mut self, value: &mut glam::Vec4) -> Response {
        input::Vec4Input::new(value).ui(self)
    }

    fn vec3_input(&mut self, value: &mut glam::Vec3) -> Response {
        input::Vec3Input::new(value).ui(self)
    }
}
