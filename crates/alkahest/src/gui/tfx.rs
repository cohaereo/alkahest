use alkahest_renderer::{
    renderer::RendererShared,
    tfx::externs::{TextureView, TfxExpressionErrorType, TfxExtern},
    ColorExt,
};
use egui::{Color32, Context, RichText};
use egui_extras::{Column, TableBuilder};
use glam::{EulerRot, Quat, Vec4};
use itertools::Itertools;
use winit::window::Window;

use crate::{
    gui::{
        context::{GuiCtx, GuiView, HiddenWindows, ViewResult},
        UiExt,
    },
    resources::Resources,
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
        let renderer = resources.get::<RendererShared>();
        let externs = &mut renderer.data.lock().externs;

        let mut windows = resources.get_mut::<HiddenWindows>();
        egui::Window::new("TFX Expression Debugger")
            .default_size([640., 720.])
            .open(&mut windows.tfx_extern_debugger)
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
                            let errors = externs.errors.read();
                            let mut errors = errors.iter().collect_vec();
                            errors.sort_by_key(|(msg, _)| *msg);

                            for (message, error) in errors {
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
                                                .color(
                                                    background_color.text_color_for_background(),
                                                ),
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

        // (!open).then_some(ViewResult::Close)
        None
    }
}

#[derive(Default)]
pub struct TfxExternEditor {
    only_show_used: bool,
}

impl GuiView for TfxExternEditor {
    fn draw(
        &mut self,
        ctx: &Context,
        _window: &Window,
        resources: &Resources,
        _gui: &GuiCtx<'_>,
    ) -> Option<ViewResult> {
        // cohae: When adding externs to this list, make sure the static values don't get reset each frame
        // Additionally, object-specific externs (such as RigidModel or SimpleGeometry) are not editable
        const SHOWN_EXTERNS: &[TfxExtern] = &[
            TfxExtern::Frame,
            // TfxExtern::View,
            // TfxExtern::Deferred,
            TfxExtern::Atmosphere,
            // TfxExtern::Mlaa,
            // TfxExtern::Msaa,
            TfxExtern::Hdao,
            // TfxExtern::Ssao,
            // TfxExtern::Postprocess,
            TfxExtern::Transparent,
            // TfxExtern::Vignette,
            TfxExtern::GlobalLighting,
            // TfxExtern::ShadowMask,
            // TfxExtern::Fxaa,
            // TfxExtern::Smaa,
            // TfxExtern::DepthOfField,
            // TfxExtern::MinmaxDepth,
            TfxExtern::Water,
            // TfxExtern::GammaControl,
            // TfxExtern::Distortion,
            // TfxExtern::VolumetricsPass,
            // TfxExtern::TemporalReprojection,
            // TfxExtern::Ssao3d,
            // TfxExtern::WaterDisplacement,
            // TfxExtern::PatternBlending,
            TfxExtern::DeferredLight,
            TfxExtern::DeferredShadow,
            TfxExtern::SpeedtreePlacements,
            TfxExtern::DecoratorWind,
        ];

        let renderer = resources.get::<RendererShared>();
        let externs = &mut renderer.data.lock().externs;

        // let mut open = true;
        let mut windows = resources.get_mut::<HiddenWindows>();
        egui::Window::new("TFX Extern Editor")
            .default_size([640., 720.])
            .open(&mut windows.tfx_extern_editor)
            .show(ctx, |ui| {
                egui::ScrollArea::new([false, true]).show(ui, |ui| {
                    for &ext in SHOWN_EXTERNS {
                        let x = externs.get_extern_editable(ext);
                        ui.add_enabled_ui(x.is_some(), |ui| {
                            let suffix = if x.is_some() { "" } else { " (not set)" };
                            ui.collapsing(format!("{ext:?}{suffix}"), |ui| {
                                if let Some(x) = x {
                                    let fields = x.field_names();
                                    for &field in fields {
                                        let mut f = x.field_mut(field).unwrap();
                                        ui.horizontal(|ui| {
                                            ui.strong(format!("{field}: "));

                                            if let Some(v) = f.get_mut::<Vec4>() {
                                                ui.vec4_input(v);
                                            }

                                            if let Some(v) = f.get_mut::<Quat>() {
                                                let mut rot = v.to_euler(EulerRot::XYZ);
                                                rot.0 = rot.0.to_degrees();
                                                rot.1 = rot.1.to_degrees();
                                                rot.2 = rot.2.to_degrees();
                                                ui.horizontal(|ui| {
                                                    ui.add(
                                                        egui::DragValue::new(&mut rot.0).speed(0.2),
                                                    );
                                                    ui.add(
                                                        egui::DragValue::new(&mut rot.1).speed(0.2),
                                                    );
                                                    ui.add(
                                                        egui::DragValue::new(&mut rot.2).speed(0.2),
                                                    );
                                                });

                                                *v = Quat::from_euler(
                                                    EulerRot::XYZ,
                                                    rot.0.to_radians(),
                                                    rot.1.to_radians(),
                                                    rot.2.to_radians(),
                                                );
                                            }

                                            // if let Some(v) = f.get::<Mat4>() {
                                            //     ui.label(format!("{:#?}", v));
                                            // }

                                            if let Some(v) = f.get_mut::<f32>() {
                                                ui.add(egui::DragValue::new(v).speed(0.01));
                                            }

                                            if let Some(v) = f.get::<TextureView>() {
                                                ui.label(format!("{:?}", v));
                                            }
                                        });
                                    }
                                }
                            });
                        });
                    }

                    ui.collapsing("Global Channels", |ui| {
                        ui.checkbox(&mut self.only_show_used, "Only show used");
                        for (i, value) in externs.global_channels.iter_mut().enumerate() {
                            let times_used = externs.global_channels_used.read()[i];
                            if self.only_show_used && times_used == 0 {
                                continue;
                            }

                            ui.horizontal(|ui| {
                                ui.strong(format!("channel {i}: "));
                                ui.vec4_input(value);
                                ui.label(format!("(used {times_used} times)"))
                            });
                        }

                        externs
                            .global_channels_used
                            .write()
                            .iter_mut()
                            .for_each(|x| *x = 0);
                    });
                });
            });

        // (!open).then_some(ViewResult::Close)
        None
    }
}
