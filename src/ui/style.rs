use egui::{style::Interaction, *};

pub fn gui_style() -> Style {
    Style {
        visuals: Visuals {
            override_text_color: Some(Color32::WHITE),
            window_fill: Color32::from_rgb(19, 19, 22),
            panel_fill: Color32::from_rgb(19, 19, 22),
            // window_fill: Color32::WHITE,
            // panel_fill: Color32::WHITE,
            ..Visuals::dark()
        },
        spacing: Spacing {
            button_padding: egui::vec2(25.0, 20.0),
            item_spacing: egui::vec2(20.0, 10.0),
            ..Default::default()
        },
        interaction: Interaction {
            selectable_labels: false,
            show_tooltips_only_when_still: false,
            ..Default::default()
        },
        ..Default::default()
    }
}
