use crate::input::InputState;
use crate::overlays::gui::OverlayProvider;
use crate::resources::Resources;

use egui::{Color32, RichText};
use lazy_static::lazy_static;
use ringbuffer::{AllocRingBuffer, RingBuffer};
use std::fmt::Debug;
use std::sync::Arc;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;
use winit::event::VirtualKeyCode;
use winit::window::Window;

// ! Do NOT swap this RwLock to our own implementation, as it will cause infinite recursion
lazy_static! {
    static ref MESSAGE_BUFFER: Arc<parking_lot::RwLock<AllocRingBuffer<CapturedEvent>>> =
        Arc::new(parking_lot::RwLock::new(AllocRingBuffer::new(8192)));
}

/// Tracing layer to capture events
pub struct ConsoleLogLayer;

struct ConsoleLogVisitor {
    fields: Vec<(String, String)>,
}

impl Visit for ConsoleLogVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.fields
            .push((field.name().to_string(), format!("{value:?}")))
    }
}

struct CapturedEvent {
    level: Level,
    target: String,
    message: String,
}

impl<S> Layer<S> for ConsoleLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = ConsoleLogVisitor { fields: vec![] };

        event.record(&mut visitor);
        let mut message = None;
        for (f, v) in visitor.fields {
            if f.as_str() == "message" {
                message = Some(v);
            }
        }

        if let Some(message) = message {
            MESSAGE_BUFFER.write().push(CapturedEvent {
                level: *event.metadata().level(),
                target: event.metadata().target().to_string(),
                message,
            })
        }
    }
}

pub struct ConsoleOverlay {
    pub command_buffer: String,
    pub autoscroll: bool,
    pub focus_input: bool,
    pub open: bool,
}

impl Default for ConsoleOverlay {
    fn default() -> Self {
        Self {
            command_buffer: "".to_string(),
            autoscroll: true,
            focus_input: false,
            open: false,
        }
    }
}

impl OverlayProvider for ConsoleOverlay {
    fn draw(&mut self, ctx: &egui::Context, _window: &Window, resources: &mut Resources) {
        let input = resources.get::<InputState>().unwrap();
        if (input.is_key_pressed(VirtualKeyCode::Grave) || input.is_key_pressed(VirtualKeyCode::F1))
            && !self.open
        {
            self.open = true;
            self.focus_input = true;
        }

        if self.open {
            let response = egui::Window::new("Console")
                .open(&mut self.open)
                .show(ctx, |ui| {
                    let c = MESSAGE_BUFFER.read();
                    // ui.child_window("Console log")
                    //     // .flags(WindowFlags::NO_TITLE_BAR)
                    //     .size([ui.window_size()[0] - 16.0, ui.window_size()[1] - 58.0])
                    //     .build(|| {
                    //         is_focused |= ui.is_window_focused();
                    //         for e in c.iter() {
                    //             let level_color = match e.level {
                    //                 Level::TRACE => [0.8, 0.4, 0.8, 1.0],
                    //                 Level::DEBUG => [0.35, 0.35, 1.0, 1.0],
                    //                 Level::INFO => [0.25, 1.0, 0.25, 1.0],
                    //                 Level::WARN => [1.0, 1.0, 0.15, 1.0],
                    //                 Level::ERROR => [1.0, 0.15, 0.15, 1.0],
                    //             };
                    //             ui.text_colored(level_color, format!("{:5} ", e.level));
                    //             ui.same_line();
                    //             ui.text_colored(
                    //                 [0.6, 0.6, 0.6, 1.0],
                    //                 format!("{}: ", e.target),
                    //             );
                    //             ui.same_line();
                    //             ui.text(&e.message);
                    //         }

                    //         if self.autoscroll {
                    //             ui.set_scroll_here_y();
                    //         }
                    //     });

                    egui::ScrollArea::new([false, true]).show_rows(
                        ui,
                        14.0,
                        c.len(),
                        |ui, row_range| {
                            for row in row_range {
                                let event = &c[row as isize];
                                let level_color = match event.level {
                                    Level::TRACE => [0.8, 0.4, 0.8],
                                    Level::DEBUG => [0.35, 0.35, 1.0],
                                    Level::INFO => [0.25, 1.0, 0.25],
                                    Level::WARN => [1.0, 1.0, 0.15],
                                    Level::ERROR => [1.0, 0.15, 0.15],
                                };
                                let level_color = Color32::from_rgb(
                                    (level_color[0] * 255.0) as u8,
                                    (level_color[1] * 255.0) as u8,
                                    (level_color[2] * 255.0) as u8,
                                );

                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!("{:5} ", event.level))
                                            .color(level_color)
                                            .monospace(),
                                    );
                                    ui.label(
                                        RichText::new(format!("{}: ", event.target))
                                            .color(Color32::GRAY)
                                            .monospace(),
                                    );
                                    ui.label(RichText::new(&event.message).monospace());
                                });
                            }
                        },
                    );

                    if self.focus_input {
                        ctx.memory_mut(|m| m.request_focus(egui::Id::new("console_input_line")));
                        self.focus_input = false;
                    }

                    if egui::TextEdit::singleline(&mut self.command_buffer)
                        .id(egui::Id::new("console_input_line"))
                        .show(ui)
                        .response
                        .lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        self.command_buffer.clear();
                        self.focus_input = true;
                    }
                });

            if let Some(response) = response {
                if (input.is_key_pressed(VirtualKeyCode::Grave)
                    || input.is_key_pressed(VirtualKeyCode::F1))
                    && !response.response.has_focus()
                {
                    self.focus_input = true;
                }

                if response.response.has_focus() && (input.is_key_pressed(VirtualKeyCode::Escape))
                // || input.is_key_pressed(VirtualKeyCode::Grave)
                // || input.is_key_pressed(VirtualKeyCode::F1))
                {
                    self.open = false;
                }
            }
        }
    }
}
