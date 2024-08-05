use alkahest_renderer::icons::{
    ICON_ALPHA_A_BOX_OUTLINE, ICON_ALPHA_D_BOX_OUTLINE, ICON_ALPHA_E_BOX_OUTLINE,
    ICON_ALPHA_G_BOX_OUTLINE, ICON_ALPHA_H_BOX_OUTLINE, ICON_ALPHA_I_BOX_OUTLINE,
    ICON_ALPHA_Q_BOX_OUTLINE, ICON_ALPHA_S_BOX_OUTLINE, ICON_ALPHA_W_BOX_OUTLINE,
    ICON_APPLE_KEYBOARD_SHIFT, ICON_ARROW_ALL, ICON_KEYBOARD_SPACE, ICON_MOUSE_LEFT_CLICK_OUTLINE,
    ICON_MOUSE_RIGHT_CLICK_OUTLINE, ICON_NUMERIC_1_BOX_OUTLINE, ICON_NUMERIC_2_BOX_OUTLINE,
    ICON_NUMERIC_3_BOX_OUTLINE, ICON_NUMERIC_4_BOX_OUTLINE,
};
use egui::{vec2, Color32, RichText, Vec2};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use winit::window::Window;

use crate::{
    gui::context::{GuiCtx, GuiView, HiddenWindows, ViewResult},
    resources::AppResources,
    util::{consts, consts::CHANGELOG_MD},
};

mod help;
mod utility;

#[derive(Default)]
pub struct MenuBar {
    controls_open: bool,
    changelog_open: bool,
    about_open: bool,
    markdown_cache: CommonMarkCache,
}

macro_rules! control_section_title {
    ($ui:expr, $title:expr) => {{
        $ui.separator();
        $ui.label(RichText::new($title).size(20.0).strong());

        $ui.end_row();
    }};
}

macro_rules! control_description {
    ($ui:expr, $control:expr, $description:expr) => {{
        $ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            ui.label(RichText::new($control).size(24.0).strong());
        });

        $ui.label($description);
        $ui.end_row();
    }};
}

impl GuiView for MenuBar {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &Window,
        resources: &AppResources,
        _gui: &GuiCtx<'_>,
    ) -> Option<ViewResult> {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Utility", |ui| {
                    self.utility_menu(ui, resources);
                });

                ui.menu_button("View", |ui| {
                    let mut windows = resources.get_mut::<HiddenWindows>();
                    windows.tfx_extern_debugger ^= ui
                        .selectable_label(windows.tfx_extern_debugger, "TFX Extern Debugger")
                        .clicked();
                    windows.tfx_extern_editor ^= ui
                        .selectable_label(windows.tfx_extern_editor, "TFX Extern Editor")
                        .clicked();
                    windows.cpu_profiler ^= ui
                        .selectable_label(windows.cpu_profiler, "Profiler")
                        .clicked();
                });

                ui.menu_button("Help", |ui| {
                    self.help_menu(ui, resources);
                });
            });
        });

        self.change_log(ctx);
        self.about(ctx);
        self.controls(ctx);

        None
    }
}

impl MenuBar {
    pub fn change_log(&mut self, ctx: &egui::Context) {
        egui::Window::new("Changelog")
            .open(&mut self.changelog_open)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    CommonMarkViewer::new("changelog_md").show(
                        ui,
                        &mut self.markdown_cache,
                        CHANGELOG_MD,
                    );
                })
            });
    }
    pub fn about(&mut self, ctx: &egui::Context) {
        egui::Window::new("About")
            .open(&mut self.about_open)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .inner_margin(Vec2::splat(16.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.allocate_ui(vec2(128.0, 128.0), |ui| {
                                ui.add(egui::Image::new(egui::include_image!(
                                    "../../../../alkahest/assets/icons/alkahest.png"
                                )));
                            });
                            ui.add_space(16.0);
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.heading(
                                        RichText::new("Alkahest").strong().color(Color32::WHITE),
                                    );
                                    ui.heading(format!("- v{}", consts::VERSION));
                                });
                                ui.separator();
                                ui.label(format!("Revision {}", consts::GIT_HASH));
                                ui.label(format!("Built on {}", consts::BUILD_DATE));
                                if let Ok(v) = rustc_version::version_meta() {
                                    ui.add_space(8.0);
                                    ui.label(format!("rustc {}+{:?}", v.semver, v.channel));
                                }
                            })
                        });
                    })
            });
    }

    pub fn controls(&mut self, ctx: &egui::Context) {
        egui::Window::new("Controls")
            .open(&mut self.controls_open)
            .auto_sized()
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("controls")
                        .min_row_height(30.0)
                        .min_col_width(200.0)
                        .show(ui, |ui| {
                            control_section_title!(ui, "Movement");

                            control_description!(
                                ui,
                                format!("{}+{}", ICON_MOUSE_RIGHT_CLICK_OUTLINE, ICON_ARROW_ALL),
                                "Adjust Camera Direction"
                            );

                            control_description!(
                                ui,
                                format!(
                                    "{}/{}/{}/{}",
                                    ICON_ALPHA_W_BOX_OUTLINE,
                                    ICON_ALPHA_S_BOX_OUTLINE,
                                    ICON_ALPHA_A_BOX_OUTLINE,
                                    ICON_ALPHA_D_BOX_OUTLINE
                                ),
                                "Move Camera Forwards/Backwards/Left/Right"
                            );

                            control_description!(
                                ui,
                                format!(
                                    "{}/{}",
                                    ICON_ALPHA_Q_BOX_OUTLINE, ICON_ALPHA_E_BOX_OUTLINE,
                                ),
                                "Move Camera Down/Up"
                            );

                            control_description!(
                                ui,
                                format!(
                                    "Alt + {}/{}/{}/{}",
                                    ICON_ALPHA_W_BOX_OUTLINE,
                                    ICON_ALPHA_S_BOX_OUTLINE,
                                    ICON_ALPHA_A_BOX_OUTLINE,
                                    ICON_ALPHA_D_BOX_OUTLINE
                                ),
                                "Move Camera in Horizontal Plain"
                            );

                            control_description!(
                                ui,
                                format!(
                                    "Alt + {}/{}",
                                    ICON_ALPHA_Q_BOX_OUTLINE, ICON_ALPHA_E_BOX_OUTLINE,
                                ),
                                "Move Camera Down/Up in Absolute Coordinates"
                            );

                            control_description!(ui, "Ctrl", "Decrease Movement speed");

                            control_description!(
                                ui,
                                format!("{} Shift", ICON_APPLE_KEYBOARD_SHIFT),
                                "Increase Movement speed"
                            );

                            control_description!(
                                ui,
                                ICON_KEYBOARD_SPACE,
                                "Increase Movement speed a lot"
                            );

                            control_description!(
                                ui,
                                format!(
                                    "{} Shift + {}",
                                    ICON_APPLE_KEYBOARD_SHIFT, ICON_KEYBOARD_SPACE
                                ),
                                "We're gonna have to go right to... LUDICROUS SPEED"
                            );

                            control_description!(
                                ui,
                                ICON_ALPHA_G_BOX_OUTLINE,
                                "Move Camera to Position of Gaze"
                            );

                            control_section_title!(ui, "Object Interactions");

                            control_description!(
                                ui,
                                ICON_MOUSE_LEFT_CLICK_OUTLINE,
                                "Select Object"
                            );

                            control_description!(
                                ui,
                                format!("{}", ICON_NUMERIC_1_BOX_OUTLINE),
                                "Selection Tool"
                            );

                            control_description!(
                                ui,
                                format!("{}", ICON_NUMERIC_2_BOX_OUTLINE),
                                "Translation Tool"
                            );

                            control_description!(
                                ui,
                                format!("{}", ICON_NUMERIC_3_BOX_OUTLINE),
                                "Rotation Tool"
                            );

                            control_description!(
                                ui,
                                format!("{}", ICON_NUMERIC_4_BOX_OUTLINE),
                                "Scale Tool"
                            );

                            //
                            // control_description!(
                            //     ui,
                            //     ICON_ALPHA_F_BOX_OUTLINE,
                            //     "Focus on Selected Object"
                            // );

                            control_description!(
                                ui,
                                ICON_ALPHA_H_BOX_OUTLINE,
                                "Toggle Hide Selected Object"
                            );

                            control_description!(
                                ui,
                                format!("Alt + {}", ICON_ALPHA_H_BOX_OUTLINE),
                                "Unhide All Objects"
                            );

                            control_description!(
                                ui,
                                format!(
                                    "{} Shift + {}",
                                    ICON_APPLE_KEYBOARD_SHIFT, ICON_ALPHA_H_BOX_OUTLINE
                                ),
                                "Hide All Unselected Objects"
                            );

                            control_description!(
                                ui,
                                format!(
                                    "Ctrl + {} Shift + {}",
                                    ICON_APPLE_KEYBOARD_SHIFT, ICON_ALPHA_H_BOX_OUTLINE
                                ),
                                "Deselect All Objects"
                            );

                            // control_description!(
                            //     ui,
                            //     format!("{} Shift + Delete", ICON_APPLE_KEYBOARD_SHIFT),
                            //     "Delete Selected Object (if allowed)"
                            // );
                            //
                            // control_description!(
                            //     ui,
                            //     ICON_ARROW_DOWN_BOLD_BOX_OUTLINE,
                            //     "Select 'Next' Object"
                            // );
                            //
                            // control_description!(
                            //     ui,
                            //     ICON_ARROW_UP_BOLD_BOX_OUTLINE,
                            //     "Select 'Previous' Object"
                            // );

                            control_section_title!(ui, "Map Changing");

                            control_description!(
                                ui,
                                ICON_ALPHA_I_BOX_OUTLINE,
                                "Swap to Previous Map"
                            );

                            control_description!(ui, "Page Up", "Swap to Previous Map in List");

                            control_description!(ui, "Page Down", "Swap to Next Map in List");
                        });
                });
            });
    }
}
