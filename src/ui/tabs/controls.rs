use egui::{Color32, FontId, RichText, TextStyle, Ui, WidgetText};
use google_material_symbols::GoogleMaterialSymbols;

use crate::ui::destiny_icons::*;
pub struct ControlsTab;

fn control_section_title(ui: &mut Ui, title: impl Into<String>) {
    ui.separator();
    ui.label(RichText::new(title).size(20.0).strong());

    ui.end_row();
}

fn control_description(
    ui: &mut Ui,
    control: impl Into<String>,
    description: impl Into<WidgetText>,
) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
        ui.label(RichText::new(control).size(24.0).strong());
    });

    ui.label(description);
    ui.end_row();
}

fn control_description_two_layer(
    ui: &mut Ui,
    layer1: impl Into<String>,
    color: Color32,
    layer2: impl Into<String>,
    description: impl Into<WidgetText>,
) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
        let mut job = egui::text::LayoutJob::default();

        egui::RichText::new(layer1)
            .color(color)
            .size(24.0)
            .strong()
            .append_to(
                &mut job,
                ui.style(),
                egui::FontSelection::Default,
                egui::Align::Center,
            );

        egui::RichText::new(layer2).size(24.0).strong().append_to(
            &mut job,
            ui.style(),
            egui::FontSelection::Default,
            egui::Align::Center,
        );

        // Render both overlapping glyphs as ONE single widget
        ui.label(job);
    });

    ui.label(description);
    ui.end_row();
}

const MOUSE_BLUE: Color32 = Color32::from_rgb(0x43, 0xa4, 0xe0);

impl ControlsTab {
    pub fn ui(ui: &mut Ui) {
        ui.style_mut()
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(16.0));

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("controls")
                .min_row_height(30.0)
                .min_col_width(200.0)
                .show(ui, |ui| {
                    control_section_title(ui, "Movement");

                    control_description_two_layer(
                        ui,
                        ICON_MOUSE2_BUTTON,
                        MOUSE_BLUE,
                        format!("{ICON_MOUSE2}+{}", GoogleMaterialSymbols::DragPan),
                        "Adjust Camera Direction",
                    );

                    control_description(
                        ui,
                        format!(
                            "{ICON_UPPERCASE_W}/{ICON_UPPERCASE_S}/{ICON_UPPERCASE_A}/\
                             {ICON_UPPERCASE_D}"
                        ),
                        "Move Camera Forwards/Backwards/Left/Right",
                    );

                    control_description(
                        ui,
                        format!("{ICON_UPPERCASE_Q}/{ICON_UPPERCASE_E}"),
                        "Move Camera Down/Up",
                    );

                    control_description(
                        ui,
                        format!(
                            "{ICON_ALT_LEFT}+{ICON_UPPERCASE_W}/{ICON_UPPERCASE_S}/\
                             {ICON_UPPERCASE_A}/{ICON_UPPERCASE_D}"
                        ),
                        "Move Camera in Horizontal Plain",
                    );

                    control_description(
                        ui,
                        format!("{ICON_ALT_LEFT}+{ICON_UPPERCASE_Q}/{ICON_UPPERCASE_E}"),
                        "Move Camera Down/Up in Absolute Coordinates",
                    );

                    control_description(ui, ICON_CTRL_LEFT, "Decrease Movement speed");

                    control_description(
                        ui,
                        format!("{ICON_SHIFT_LEFT} Shift"),
                        "Increase Movement speed",
                    );

                    control_description(ui, ICON_SPACEBAR, "Increase Movement speed a lot");

                    control_description(
                        ui,
                        format!("{ICON_SHIFT_LEFT} Shift + {ICON_SPACEBAR}"),
                        "We're gonna have to go right to... LUDICROUS SPEED",
                    );

                    control_description_two_layer(
                        ui,
                        ICON_MOUSEWHEEL_UP_BUTTON,
                        MOUSE_BLUE,
                        ICON_MOUSEWHEEL_UP,
                        "Increase Movement speed multiplier",
                    );

                    control_description_two_layer(
                        ui,
                        ICON_MOUSEWHEEL_DOWN_BUTTON,
                        MOUSE_BLUE,
                        ICON_MOUSEWHEEL_DOWN,
                        "Decrease Movement speed multiplier",
                    );

                    control_description(
                        ui,
                        format!("{ICON_HOME} Home"),
                        "Go to a random (default) spawn point",
                    );

                    control_description(ui, ICON_UPPERCASE_G, "Move Camera to Position of Gaze");

                    control_description(ui, ICON_UPPERCASE_C, "Toggle Crosshair");

                    // control_section_title(ui, "Object Interactions");

                    // control_description(
                    //     ui,
                    //     (ICON_MOUSE1_BUTTON, MOUSE_BLUE),
                    //     ICON_MOUSE1,
                    //     "Select Object"
                    // );

                    // control_description(
                    //     ui,
                    //     ICON_NUM_1,
                    //     "Selection Tool"
                    // );

                    // control_description(
                    //     ui,
                    //     ICON_NUM_2,
                    //     "Translation Tool"
                    // );

                    // control_description(
                    //     ui,
                    //     ICON_NUM_3,
                    //     "Rotation Tool"
                    // );

                    // control_description(
                    //     ui,
                    //     ICON_NUM_4,
                    //     "Scale Tool"
                    // );

                    //
                    // control_description(
                    //     ui,
                    //     ICON_UPPERCASE_F,
                    //     "Focus on Selected Object"
                    // );

                    // control_description(
                    //     ui,
                    //     ICON_UPPERCASE_H,
                    //     "Toggle Hide Selected Object"
                    // );

                    // control_description(
                    //     ui,
                    //     format("{ICON_ALT_LEFT} + {ICON_UPPERCASE_H}"),
                    //     "Unhide All Objects"
                    // );

                    // control_description(
                    //     ui,
                    //     format(
                    //         "{ICON_SHIFT_LEFT} Shift + {ICON_UPPERCASE_H}",
                    //     ),
                    //     "Hide All Unselected Objects"
                    // );

                    // control_description(
                    //     ui,
                    //     format(
                    //         "{ICON_CTRL_LEFT}+{ICON_SHIFT_LEFT} Shift +{ICON_UPPERCASE_H}",
                    //     ),
                    //     "Deselect All Objects"
                    // );

                    // control_description(
                    //     ui,
                    //     ICON_UP,
                    //     "Select Parent"
                    // );

                    // control_description(
                    //     ui,
                    //     ICON_DOWN,
                    //     "Select First Child"
                    // );

                    // control_description(
                    //     ui,
                    //     ICON_LEFT,
                    //     "Select Previous Child"
                    // );

                    // control_description(
                    //     ui,
                    //     ICON_RIGHT,
                    //     "Select Next Child"
                    // );
                    // control_description(
                    //      ui,
                    //      format("{ICON_SHIFT_LEFT} Shift + Delete"),
                    //      "Delete Selected Object (if allowed)"
                    //  );

                    //  control_description(
                    //      ui,
                    //      ICON_DOWN,
                    //      "Select 'Next' Object"
                    //  );

                    // control_description(
                    //     ui,
                    //     ICON_UP,
                    //     "Select 'Previous' Object"
                    // );

                    control_section_title(ui, "Map Changing");

                    control_description(ui, ICON_UPPERCASE_I, "Swap to Previous Map");

                    control_description(
                        ui,
                        format!("{ICON_PAGE_UP} Page Up"),
                        "Swap to Previous Map in List",
                    );

                    control_description(
                        ui,
                        format!("{ICON_PAGE_DOWN} Page Down"),
                        "Swap to Next Map in List",
                    );

                    // control_section_title(ui, "Route Editing");

                    // control_description(
                    //     ui,
                    //     ICON_PLUS,
                    //     "Add node at end of route, or after selected node"
                    // );

                    // control_description(
                    //     ui,
                    //     ICON_HYPHEN,
                    //     "Add node before selected node"
                    // );
                });
        });
    }
}
