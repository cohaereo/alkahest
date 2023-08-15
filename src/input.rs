use tracing::warn;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};

pub type Key = VirtualKeyCode;
const WINIT_KEY_COUNT: usize = Key::Cut as usize + 1;

#[derive(PartialEq, Eq, Default, Copy, Clone)]
pub enum KeyState {
    #[default]
    Up,
    Down,
    Repeated,
}

#[derive(PartialEq, Eq)]
pub enum MouseButton {
    /// Alias for Mouse1
    Left,
    /// Alias for Mouse2
    Right,
    /// Alias for Mouse3
    Middle,
    // /// Alias for Mouse5
    // Forward,
    // /// Alias for Mouse4
    // Back,
}

pub struct InputState {
    keys: [KeyState; WINIT_KEY_COUNT],

    ctrl: bool,
    alt: bool,
    shift: bool,

    /// Left mouse button
    mouse1: bool,
    /// Right mouse button
    mouse2: bool,
    /// Scroll wheel button
    mouse3: bool,
    // /// 'Back' side button
    // mouse4: bool,
    // /// 'Forward' side button
    // mouse5: bool,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            keys: [KeyState::Up; WINIT_KEY_COUNT],
            ctrl: false,
            alt: false,
            shift: false,
            mouse1: false,
            mouse2: false,
            mouse3: false,
            // mouse4: false,
            // mouse5: false,
        }
    }
}

impl InputState {
    /// Handles winit events and updates the state accordingly
    pub fn handle_event(&mut self, event: &Event<'_, ()>) {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode,
                            state,
                            ..
                        },
                    ..
                } => {
                    if let Some(vk) = virtual_keycode {
                        let mut key = &mut self.keys[*vk as usize];
                        match state {
                            winit::event::ElementState::Pressed => match *key {
                                KeyState::Up => *key = KeyState::Down,
                                KeyState::Down => *key = KeyState::Repeated,
                                KeyState::Repeated => {}
                            },
                            winit::event::ElementState::Released => {
                                *key = KeyState::Up;
                            }
                        }
                    }
                }
                WindowEvent::ModifiersChanged(modifiers) => {
                    self.ctrl = modifiers.ctrl();
                    self.alt = modifiers.alt();
                    self.shift = modifiers.shift();
                }
                // WindowEvent::MouseWheel { device_id, delta, phase, modifiers } => todo!(),
                WindowEvent::MouseInput { state, button, .. } => match button {
                    winit::event::MouseButton::Left => {
                        self.mouse1 = *state == ElementState::Pressed
                    }
                    winit::event::MouseButton::Right => {
                        self.mouse2 = *state == ElementState::Pressed
                    }
                    winit::event::MouseButton::Middle => {
                        self.mouse3 = *state == ElementState::Pressed
                    }
                    winit::event::MouseButton::Other(_) => {}
                },
                _ => {}
            },
            _ => {}
        }
    }

    pub fn key_state(&self, vk: Key) -> KeyState {
        self.keys[vk as usize]
    }

    /// Returns true if the key is being held.
    pub fn is_key_down(&self, vk: Key) -> bool {
        matches!(self.key_state(vk), KeyState::Down | KeyState::Repeated)
    }

    /// Returns true if the key was pressed (went from !down to down
    pub fn is_key_pressed(&self, vk: Key) -> bool {
        self.key_state(vk) == KeyState::Down
    }

    pub fn ctrl(&self) -> bool {
        self.ctrl
    }

    pub fn alt(&self) -> bool {
        self.alt
    }

    pub fn shift(&self) -> bool {
        self.shift
    }

    pub fn is_mouse_down(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.mouse1,
            MouseButton::Right => self.mouse2,
            MouseButton::Middle => self.mouse3,
            // MouseButton::Forward => self.mouse5,
            // MouseButton::Back => self.mouse4,
        }
    }

    pub fn mouse_left(&self) -> bool {
        self.is_mouse_down(MouseButton::Left)
    }

    pub fn mouse_right(&self) -> bool {
        self.is_mouse_down(MouseButton::Right)
    }

    pub fn mouse_middle(&self) -> bool {
        self.is_mouse_down(MouseButton::Middle)
    }

    // pub fn mouse_back(&self) -> bool {
    //     self.is_mouse_down(MouseButton::Back)
    // }

    // pub fn mouse_forward(&self) -> bool {
    //     self.is_mouse_down(MouseButton::Forward)
    // }
}
