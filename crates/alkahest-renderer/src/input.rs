use winit::{
    event::{KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

pub type Key = KeyCode;
const WINIT_KEY_COUNT: usize = Key::F35 as usize + 1;

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
    #[doc(alias = "Mouse4")]
    Back,
    #[doc(alias = "Mouse5")]
    Forward,
}

pub struct InputState {
    keys: [ButtonState; WINIT_KEY_COUNT],

    ctrl: bool,
    alt: bool,
    shift: bool,

    /// Left mouse button
    mouse1: ButtonState,
    /// Right mouse button
    mouse2: ButtonState,
    /// Scroll wheel button
    mouse3: ButtonState,
    /// 'Back' side button
    mouse4: ButtonState,
    /// 'Forward' side button
    mouse5: ButtonState,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            keys: [ButtonState::Up; WINIT_KEY_COUNT],
            ctrl: false,
            alt: false,
            shift: false,
            mouse1: ButtonState::Up,
            mouse2: ButtonState::Up,
            mouse3: ButtonState::Up,
            mouse4: ButtonState::Up,
            mouse5: ButtonState::Up,
        }
    }
}

#[allow(unused)]
impl InputState {
    /// Handles winit events and updates the state accordingly
    pub fn handle_event(&mut self, event: &WindowEvent) {
        // TODO(cohae): Resolve this lint
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(vk),
                        state,
                        ..
                    },
                ..
            } => {
                let key = &mut self.keys[*vk as usize];
                match state {
                    winit::event::ElementState::Pressed => match *key {
                        ButtonState::Up => *key = ButtonState::Down,
                        ButtonState::Down => *key = ButtonState::Repeated,
                        ButtonState::Repeated => {}
                    },
                    winit::event::ElementState::Released => {
                        *key = ButtonState::Up;
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
                winit::event::MouseButton::Forward => {
                    self.mouse5 = match state {
                        winit::event::ElementState::Pressed => match self.mouse5 {
                            ButtonState::Up => ButtonState::Down,
                            ButtonState::Down => ButtonState::Repeated,
                            ButtonState::Repeated => self.mouse5,
                        },
                        winit::event::ElementState::Released => ButtonState::Up,
                    }
                }
                winit::event::MouseButton::Back => {
                    self.mouse4 = match state {
                        winit::event::ElementState::Pressed => match self.mouse4 {
                            ButtonState::Up => ButtonState::Down,
                            ButtonState::Down => ButtonState::Repeated,
                            ButtonState::Repeated => self.mouse4,
                        },
                        winit::event::ElementState::Released => ButtonState::Up,
                    }
                }
                winit::event::MouseButton::Other(_) => {}
            },
            _ => {}
        }
    }

    pub fn key_state(&self, vk: Key) -> ButtonState {
        self.keys[vk as usize]
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

    pub fn is_mouse_clicked(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.mouse1 == ButtonState::Down,
            MouseButton::Right => self.mouse2 == ButtonState::Down,
            MouseButton::Middle => self.mouse3 == ButtonState::Down,
            MouseButton::Forward => self.mouse5 == ButtonState::Down,
            MouseButton::Back => self.mouse4 == ButtonState::Down,
        }
    }

    pub fn is_mouse_down(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => matches!(self.mouse1, ButtonState::Down | ButtonState::Repeated),
            MouseButton::Right => matches!(self.mouse2, ButtonState::Down | ButtonState::Repeated),
            MouseButton::Middle => matches!(self.mouse3, ButtonState::Down | ButtonState::Repeated),
            MouseButton::Forward => {
                matches!(self.mouse5, ButtonState::Down | ButtonState::Repeated)
            }
            MouseButton::Back => matches!(self.mouse4, ButtonState::Down | ButtonState::Repeated),
        }
    }

    pub fn mouse_left_clicked(&self) -> bool {
        self.is_mouse_clicked(MouseButton::Left)
    }

    pub fn mouse_right_clicked(&self) -> bool {
        self.is_mouse_clicked(MouseButton::Right)
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

    pub fn mouse_forward(&self) -> bool {
        self.is_mouse_down(MouseButton::Forward)
    }

    pub fn mouse_back(&self) -> bool {
        self.is_mouse_down(MouseButton::Back)
    }
}
