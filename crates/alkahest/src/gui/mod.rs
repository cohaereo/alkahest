use egui::Response;

pub mod activity_select;
pub mod context;
pub mod hotkeys;
pub mod icons;
pub mod inspector;

// Custom widgets
pub mod big_button;
pub mod chip;
mod tfx;

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
}
