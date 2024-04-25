use alkahest_renderer::tfx::externs::{ExternStorage, TfxExpressionError, TfxExpressionErrorType};
use destiny_pkg::TagHash;
use egui::{Color32, Context, RichText};
use egui_extras::{Column, TableBuilder};
use winit::window::Window;

use crate::{
    gui::context::{GuiCtx, GuiView, ViewResult},
    resources::Resources,
    util::text::text_color_for_background,
};

pub struct TfxErrorViewer {
    clear_each_frame: bool,
}

impl Default for TfxErrorViewer {
    fn default() -> Self {
        Self {
            clear_each_frame: true,
        }
    }
}

impl GuiView for TfxErrorViewer {
    fn draw(
        &mut self,
        ctx: &Context,
        _window: &Window,
        resources: &Resources,
        _gui: &GuiCtx<'_>,
    ) -> Option<ViewResult> {
        let externs = resources.get::<ExternStorage>();

        let mut open = true;
        egui::Window::new("TFX Expression Debugger")
            .default_size([640., 720.])
            .open(&mut open)
            .show(ctx, |ui| {
                ui.checkbox(&mut self.clear_each_frame, "Clear each frame");
                egui::ScrollArea::new([false, true]).show(ui, |ui| {
                    TableBuilder::new(ui)
                        .column(Column::initial(128.0).resizable(true))
                        .column(Column::remainder())
                        .striped(true)
                        .header(10.0, |mut header| {
                            header.col(|ui| {
                                ui.strong("Level");
                            });
                            header.col(|ui| {
                                ui.strong("Message");
                            });
                        })
                        .body(|mut body| {
                            for (message, error) in externs.errors.read().iter() {
                                body.row(20.0, |mut row| {
                                    row.col(|ui| {
                                        let (label, background_color) = match error.error_type {
                                            TfxExpressionErrorType::Unimplemented { .. } => {
                                                // if partial {
                                                //     ("STUBBED", Color32::YELLOW)
                                                // } else {
                                                ("UNIMPLEMENTED", Color32::RED)
                                                // }
                                            }
                                            TfxExpressionErrorType::InvalidType(_) => {
                                                ("INVALID_TYPE", Color32::DARK_RED)
                                            }

                                            TfxExpressionErrorType::ExternNotSet(_) => {
                                                ("EXTERN_NOT_SET", Color32::DARK_RED)
                                            }
                                        };

                                        ui.label(
                                            RichText::new(label)
                                                .strong()
                                                .background_color(background_color)
                                                .color(text_color_for_background(background_color)),
                                        );
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("{} ({}x)", message, error.repeat_count));
                                    });
                                });
                            }
                        });
                });
            });

        if self.clear_each_frame {
            externs.errors.write().clear();
        }

        (!open).then_some(ViewResult::Close)
    }
}
