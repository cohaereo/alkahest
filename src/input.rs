use egui::ahash::HashMap;
use winit::event::WindowEvent;
pub use winit::keyboard::Key;

#[derive(PartialEq, Eq, Default, Copy, Clone)]
pub enum ButtonState {
    #[default]
    Up,
    Down,
    #[doc(alias = "Held")]
    Repeated,
}

#[allow(unused)]
#[derive(PartialEq, Eq)]
pub enum MouseButton {
    #[doc(alias = "Mouse1")]
    Left,
    #[doc(alias = "Mouse2")]
    Right,
    #[doc(alias = "Mouse3")]
    Middle,
    // /// Alias for Mouse5
    // Forward,
    // /// Alias for Mouse4
    // Back,
}

pub struct InputState {
    keys: HashMap<Key, ButtonState>,

    ctrl: bool,
    alt: bool,
    shift: bool,

    /// Left mouse button
    mouse1: ButtonState,
    /// Right mouse button
    mouse2: ButtonState,
    /// Scroll wheel button
    mouse3: ButtonState,
    // /// 'Back' side button
    // mouse4: bool,
    // /// 'Forward' side button
    // mouse5: bool,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            keys: HashMap::default(),
            ctrl: false,
            alt: false,
            shift: false,
            mouse1: ButtonState::Up,
            mouse2: ButtonState::Up,
            mouse3: ButtonState::Up,
            // mouse4: false,
            // mouse5: false,
        }
    }
}

#[allow(unused)]
impl InputState {
    /// Handles winit events and updates the state accordingly
    pub fn handle_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        logical_key, state, ..
                    },
                ..
            } => {
                let normalized_key = normalize_key(logical_key.clone());
                let old_state = self
                    .keys
                    .get(&normalized_key)
                    .copied()
                    .unwrap_or(ButtonState::Up);
                match state {
                    winit::event::ElementState::Pressed => match old_state {
                        ButtonState::Up => {
                            self.keys.insert(normalized_key.clone(), ButtonState::Down);
                        }
                        ButtonState::Down => {
                            self.keys
                                .insert(normalized_key.clone(), ButtonState::Repeated);
                        }
                        ButtonState::Repeated => {}
                    },
                    winit::event::ElementState::Released => {
                        self.keys.insert(normalized_key.clone(), ButtonState::Up);
                    }
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.ctrl = modifiers.state().control_key();
                self.alt = modifiers.state().alt_key();
                self.shift = modifiers.state().shift_key();
            }
            // WindowEvent::MouseWheel { device_id, delta, phase, modifiers } => todo!(),
            WindowEvent::MouseInput { state, button, .. } => match button {
                winit::event::MouseButton::Left => {
                    self.mouse1 = match state {
                        winit::event::ElementState::Pressed => match self.mouse1 {
                            ButtonState::Up => ButtonState::Down,
                            ButtonState::Down => ButtonState::Repeated,
                            ButtonState::Repeated => self.mouse1,
                        },
                        winit::event::ElementState::Released => ButtonState::Up,
                    }
                }
                winit::event::MouseButton::Right => {
                    self.mouse2 = match state {
                        winit::event::ElementState::Pressed => match self.mouse2 {
                            ButtonState::Up => ButtonState::Down,
                            ButtonState::Down => ButtonState::Repeated,
                            ButtonState::Repeated => self.mouse2,
                        },
                        winit::event::ElementState::Released => ButtonState::Up,
                    }
                }
                winit::event::MouseButton::Middle => {
                    self.mouse3 = match state {
                        winit::event::ElementState::Pressed => match self.mouse3 {
                            ButtonState::Up => ButtonState::Down,
                            ButtonState::Down => ButtonState::Repeated,
                            ButtonState::Repeated => self.mouse3,
                        },
                        winit::event::ElementState::Released => ButtonState::Up,
                    }
                }
                winit::event::MouseButton::Other(_)
                | winit::event::MouseButton::Back
                | winit::event::MouseButton::Forward => {}
            },
            _ => {}
        }
    }

    pub fn key_state(&self, key: Key) -> ButtonState {
        self.keys
            .get(&normalize_key(key))
            .copied()
            .unwrap_or(ButtonState::Up)
    }

    /// Returns true if the key is being held.
    pub fn is_key_down(&self, vk: Key) -> bool {
        matches!(
            self.key_state(vk),
            ButtonState::Down | ButtonState::Repeated
        )
    }

    /// Returns true if the key was pressed (went from !down to down
    pub fn is_key_pressed(&self, vk: Key) -> bool {
        self.key_state(vk) == ButtonState::Down
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

    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.mouse1 == ButtonState::Down,
            MouseButton::Right => self.mouse2 == ButtonState::Down,
            MouseButton::Middle => self.mouse3 == ButtonState::Down,
            // MouseButton::Forward => self.mouse5,
            // MouseButton::Back => self.mouse4,
        }
    }

    pub fn is_mouse_down(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => matches!(self.mouse1, ButtonState::Down | ButtonState::Repeated),
            MouseButton::Right => matches!(self.mouse2, ButtonState::Down | ButtonState::Repeated),
            MouseButton::Middle => matches!(self.mouse3, ButtonState::Down | ButtonState::Repeated),
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

pub fn normalize_key(k: Key) -> Key {
    match k {
        Key::Character(c) => Key::Character(c.to_ascii_lowercase().into()),
        _ => k,
    }
}
