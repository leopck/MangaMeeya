use std::collections::HashMap;

use crate::{Command, GestureDirection, InputBindings, KeyCombo, MouseAction};

/// Virtual key codes matching winit/web standards.
pub mod vk {
    pub const PAGE_UP: u32 = 33;
    pub const PAGE_DOWN: u32 = 34;
    pub const END: u32 = 35;
    pub const HOME: u32 = 36;
    pub const LEFT: u32 = 37;
    pub const UP: u32 = 38;
    pub const RIGHT: u32 = 39;
    pub const DOWN: u32 = 40;
    pub const ENTER: u32 = 13;
    pub const SPACE: u32 = 32;
    pub const ESCAPE: u32 = 27;
    pub const F1: u32 = 112;
    pub const F5: u32 = 116;
    pub const F11: u32 = 122;
    pub const DELETE: u32 = 46;
    pub const KEY_O: u32 = 79;
    pub const KEY_S: u32 = 83;
    pub const KEY_Q: u32 = 81;
    pub const KEY_B: u32 = 66;
    pub const KEY_D: u32 = 68;
    pub const KEY_P: u32 = 80;
    pub const PLUS: u32 = 187;
    pub const MINUS: u32 = 189;
    pub const KEY_0: u32 = 48;
    pub const COMMA: u32 = 188;
    pub const PERIOD: u32 = 190;
    pub const BACKSPACE: u32 = 8;
    pub const TAB: u32 = 9;
}

fn key(key: u32) -> KeyCombo {
    KeyCombo {
        key,
        ctrl: false,
        shift: false,
        alt: false,
    }
}

fn ctrl(key: u32) -> KeyCombo {
    KeyCombo {
        key,
        ctrl: true,
        shift: false,
        alt: false,
    }
}

/// Create the default input bindings matching original MangaMeeya.
pub fn default_bindings() -> InputBindings {
    let mut keyboard = HashMap::new();

    // Navigation
    keyboard.insert(key(vk::PAGE_DOWN), Command::NextPage);
    keyboard.insert(key(vk::PAGE_UP), Command::PrevPage);
    keyboard.insert(key(vk::HOME), Command::FirstPage);
    keyboard.insert(key(vk::END), Command::LastPage);
    keyboard.insert(key(vk::SPACE), Command::NextPage);
    keyboard.insert(key(vk::BACKSPACE), Command::PrevPage);

    // Scrolling
    keyboard.insert(key(vk::LEFT), Command::ScrollLeft);
    keyboard.insert(key(vk::UP), Command::ScrollUp);
    keyboard.insert(key(vk::RIGHT), Command::ScrollRight);
    keyboard.insert(key(vk::DOWN), Command::ScrollDown);

    // Zoom
    keyboard.insert(key(vk::PLUS), Command::ZoomIn);
    keyboard.insert(key(vk::MINUS), Command::ZoomOut);
    keyboard.insert(key(vk::KEY_0), Command::ZoomReset);

    // View modes
    keyboard.insert(key(vk::F11), Command::ToggleFullscreen);
    keyboard.insert(key(vk::ENTER), Command::ToggleFullscreen);
    keyboard.insert(key(vk::KEY_D), Command::ToggleDualPage);

    // File operations
    keyboard.insert(ctrl(vk::KEY_O), Command::OpenFile);
    keyboard.insert(ctrl(vk::KEY_S), Command::SaveImage);
    keyboard.insert(ctrl(vk::KEY_Q), Command::Quit);

    // Bookmarks
    keyboard.insert(key(vk::KEY_B), Command::ToggleBookmark);

    // Image processing
    keyboard.insert(key(vk::COMMA), Command::RotateCcw90);
    keyboard.insert(key(vk::PERIOD), Command::RotateCw90);

    // Misc
    keyboard.insert(key(vk::KEY_P), Command::SlideShowToggle);
    keyboard.insert(key(vk::F5), Command::Refresh);
    keyboard.insert(key(vk::F1), Command::ShowSettings);
    keyboard.insert(key(vk::DELETE), Command::DeleteFile);

    // Mouse
    let mut mouse_buttons = HashMap::new();
    mouse_buttons.insert(MouseAction::WheelUp, Command::ScrollUp);
    mouse_buttons.insert(MouseAction::WheelDown, Command::ScrollDown);
    mouse_buttons.insert(MouseAction::DoubleClick, Command::ToggleFullscreen);

    // Gestures
    let mut gestures = HashMap::new();
    gestures.insert(vec![GestureDirection::Right], Command::NextPage);
    gestures.insert(vec![GestureDirection::Left], Command::PrevPage);
    gestures.insert(
        vec![GestureDirection::Up, GestureDirection::Down],
        Command::Refresh,
    );

    InputBindings {
        keyboard,
        mouse_buttons,
        gestures,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_keybindings() {
        let bindings = default_bindings();
        assert_eq!(
            bindings.keyboard.get(&key(vk::PAGE_DOWN)),
            Some(&Command::NextPage)
        );
        assert_eq!(
            bindings.keyboard.get(&key(vk::PAGE_UP)),
            Some(&Command::PrevPage)
        );
        assert_eq!(
            bindings.keyboard.get(&key(vk::HOME)),
            Some(&Command::FirstPage)
        );
        assert_eq!(
            bindings.keyboard.get(&key(vk::END)),
            Some(&Command::LastPage)
        );
    }

    #[test]
    fn test_ctrl_bindings() {
        let bindings = default_bindings();
        assert_eq!(
            bindings.keyboard.get(&ctrl(vk::KEY_O)),
            Some(&Command::OpenFile)
        );
        assert_eq!(
            bindings.keyboard.get(&ctrl(vk::KEY_Q)),
            Some(&Command::Quit)
        );
    }

    #[test]
    fn test_unbound_key() {
        let bindings = default_bindings();
        let unbound = KeyCombo {
            key: 999,
            ctrl: false,
            shift: false,
            alt: false,
        };
        assert_eq!(bindings.keyboard.get(&unbound), None);
    }

    #[test]
    fn test_mouse_bindings() {
        let bindings = default_bindings();
        assert_eq!(
            bindings.mouse_buttons.get(&MouseAction::WheelUp),
            Some(&Command::ScrollUp)
        );
        assert_eq!(
            bindings.mouse_buttons.get(&MouseAction::DoubleClick),
            Some(&Command::ToggleFullscreen)
        );
    }

    #[test]
    fn test_gesture_bindings() {
        let bindings = default_bindings();
        assert_eq!(
            bindings.gestures.get(&vec![GestureDirection::Right]),
            Some(&Command::NextPage)
        );
    }
}
