pub mod command;
pub mod gesture;
pub mod keyboard;
pub mod mouse;

use std::collections::HashMap;

pub use command::Command;

/// Combined input bindings for all input methods.
#[derive(Debug, Clone)]
pub struct InputBindings {
    pub keyboard: HashMap<KeyCombo, Command>,
    pub mouse_buttons: HashMap<MouseAction, Command>,
    pub gestures: HashMap<Vec<GestureDirection>, Command>,
}

/// A keyboard key combination.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct KeyCombo {
    pub key: u32,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

/// A mouse action identifier.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum MouseAction {
    LeftClick,
    RightClick,
    MiddleClick,
    DoubleClick,
    WheelUp,
    WheelDown,
    DragLeft,
    DragRight,
    LongPress,
}

/// Direction segment in a mouse gesture.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum GestureDirection {
    Up,
    Down,
    Left,
    Right,
}

impl Default for InputBindings {
    fn default() -> Self {
        keyboard::default_bindings()
    }
}
