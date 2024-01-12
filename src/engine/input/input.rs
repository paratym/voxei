use voxei_macros::Resource;

use crate::engine::resource::ResMut;

use super::{
    keyboard::{self, Keyboard},
    mouse::{self, Mouse},
};

#[derive(Resource)]
pub struct Input {
    keyboard: Keyboard,
    mouse: Mouse,
}

impl Input {
    pub fn new() -> Self {
        Self {
            keyboard: Keyboard::new(),
            mouse: Mouse::new(),
        }
    }

    pub fn clear_inputs(mut input: ResMut<Input>) {
        input.keyboard.clear_inputs();
        input.mouse.clear_inputs();
    }

    // Keyboard functions
    pub fn is_key_pressed(&self, key: keyboard::Key) -> bool {
        self.keyboard.is_key_pressed(key)
    }

    pub fn is_key_pressed_with_modifiers(
        &self,
        key: keyboard::Key,
        modifiers: &[keyboard::Modifier],
    ) -> bool {
        self.keyboard.is_key_pressed_with_modifiers(key, modifiers)
    }

    pub fn is_key_down_with_modifiers(
        &self,
        key: keyboard::Key,
        modifiers: &[keyboard::Modifier],
    ) -> bool {
        self.keyboard.is_key_down_with_modifiers(key, modifiers)
    }

    pub fn is_key_released_with_modifiers(
        &self,
        key: keyboard::Key,
        modifiers: &[keyboard::Modifier],
    ) -> bool {
        self.keyboard.is_key_released_with_modifiers(key, modifiers)
    }

    pub fn is_key_down(&self, key: keyboard::Key) -> bool {
        self.keyboard.is_key_down(key)
    }

    /// Returns true if the key is being viewed as held by the OS.
    /// Mainly used for text input.
    pub fn is_key_repeat(&self, key: keyboard::Key) -> bool {
        self.keyboard.is_key_repeat(key)
    }

    pub fn is_key_released(&self, key: keyboard::Key) -> bool {
        self.keyboard.is_key_released(key)
    }

    // Mouse functions
    pub fn is_mouse_button_pressed(&self, button: mouse::Button) -> bool {
        self.mouse.is_mouse_button_pressed(button)
    }

    pub fn is_mouse_button_down(&self, button: mouse::Button) -> bool {
        self.mouse.is_mouse_button_down(button)
    }

    pub fn is_mouse_button_released(&self, button: mouse::Button) -> bool {
        self.mouse.is_mouse_button_released(button)
    }

    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse.mouse_position()
    }

    pub fn mouse_delta(&self) -> (f32, f32) {
        self.mouse.mouse_delta()
    }

    pub fn keyboard(&self) -> &Keyboard {
        &self.keyboard
    }

    pub fn keyboard_mut(&mut self) -> &mut Keyboard {
        &mut self.keyboard
    }

    pub fn mouse(&self) -> &Mouse {
        &self.mouse
    }

    pub fn mouse_mut(&mut self) -> &mut Mouse {
        &mut self.mouse
    }
}
