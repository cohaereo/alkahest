use crate::input::InputState;
use crate::overlays::gui::OverlayProvider;
use crate::resources::Resources;
use imgui::Key;
use lazy_static::lazy_static;
use parking_lot::RwLock;
use ringbuffer::{AllocRingBuffer, RingBuffer};
use std::fmt::Debug;
use std::sync::Arc;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;
use winit::event::VirtualKeyCode;
use winit::window::Window;

lazy_static! {
    static ref MESSAGE_BUFFER: Arc<RwLock<AllocRingBuffer<CapturedEvent>>> =
        Arc::new(RwLock::new(AllocRingBuffer::new(8192)));
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
            match f.as_str() {
                "message" => message = Some(v),
                _ => {}
            }
        }

        if let Some(message) = message {
            MESSAGE_BUFFER.write().push(CapturedEvent {
                level: event.metadata().level().clone(),
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
    fn create_overlay(&mut self, ui: &mut imgui::Ui, _window: &Window, resources: &mut Resources) {
        let input = resources.get::<InputState>().unwrap();
        if (input.is_key_pressed(VirtualKeyCode::Grave)
            || input.is_key_pressed(VirtualKeyCode::F10))
            && !self.open
        {
            self.open = true;
            self.focus_input = true;
        }

        // TODO(cohae): Imgui does not handle the open bool all by itself??
        if self.open {
            let mut is_focused = false;
            ui.window("Console").opened(&mut self.open).build(|| {
                is_focused = ui.is_window_focused();

                let c = MESSAGE_BUFFER.read();
                ui.group(|| {
                    ui.child_window("Console log")
                        // .flags(WindowFlags::NO_TITLE_BAR)
                        .size([ui.window_size()[0] - 16.0, ui.window_size()[1] - 58.0])
                        .build(|| {
                            is_focused |= ui.is_window_focused();
                            for e in c.iter() {
                                let level_color = match e.level {
                                    Level::TRACE => [0.8, 0.4, 0.8, 1.0],
                                    Level::DEBUG => [0.35, 0.35, 1.0, 1.0],
                                    Level::INFO => [0.25, 1.0, 0.25, 1.0],
                                    Level::WARN => [1.0, 1.0, 0.15, 1.0],
                                    Level::ERROR => [1.0, 0.15, 0.15, 1.0],
                                };
                                ui.text_colored(level_color, format!("{:5} ", e.level));
                                ui.same_line();
                                ui.text_colored([0.6, 0.6, 0.6, 1.0], format!("{}: ", e.target));
                                ui.same_line();
                                ui.text(&e.message);
                            }

                            if self.autoscroll {
                                ui.set_scroll_here_y();
                            }
                        });
                });

                ui.set_next_item_width(ui.content_region_avail()[0]);
                if self.focus_input {
                    ui.set_keyboard_focus_here();
                    self.focus_input = false;
                }

                if ui
                    .input_text(" ", &mut self.command_buffer)
                    .enter_returns_true(true)
                    .build()
                {
                    self.command_buffer.clear();
                    self.focus_input = true;
                }

                if (input.is_key_pressed(VirtualKeyCode::Grave)
                    || input.is_key_pressed(VirtualKeyCode::F10))
                    && !ui.is_window_focused()
                {
                    self.focus_input = true;
                }
            });

            if is_focused && (input.is_key_pressed(VirtualKeyCode::Escape))
            // || input.is_key_pressed(VirtualKeyCode::Grave)
            // || input.is_key_pressed(VirtualKeyCode::F10))
            {
                self.open = false;
            }
        }
    }
}
