use vst3::Steinberg::VirtualKeyCodes_::*;

pub(super) fn unicode_to_string(unicode: u32) -> Option<String> {
    char::from_u32(unicode)
        .map(|char| char.to_string())
}

pub(super) fn to_key_code(code: i32) -> keyboard_types::Code {
    // Cast is needed on MacOS
    #[allow(clippy::unnecessary_cast)]
    match code as _ {
        KEY_BACK => keyboard_types::Code::Backspace,
        KEY_TAB => keyboard_types::Code::Tab,
        KEY_RETURN => keyboard_types::Code::Enter,
        KEY_ESCAPE => keyboard_types::Code::Escape,
        KEY_SPACE => keyboard_types::Code::Space,
        KEY_END => keyboard_types::Code::End,
        KEY_HOME => keyboard_types::Code::Home,
        KEY_LEFT => keyboard_types::Code::ArrowLeft,
        KEY_UP => keyboard_types::Code::ArrowUp,
        KEY_RIGHT => keyboard_types::Code::ArrowRight,
        KEY_DOWN => keyboard_types::Code::ArrowDown,
        KEY_PAGEUP => keyboard_types::Code::PageUp,
        KEY_PAGEDOWN => keyboard_types::Code::PageDown,
        KEY_ENTER => keyboard_types::Code::NumpadEnter,
        KEY_INSERT => keyboard_types::Code::Insert,
        KEY_DELETE => keyboard_types::Code::Delete,
        KEY_NUMPAD0 => keyboard_types::Code::Numpad0,
        KEY_NUMPAD1 => keyboard_types::Code::Numpad1,
        KEY_NUMPAD2 => keyboard_types::Code::Numpad2,
        KEY_NUMPAD3 => keyboard_types::Code::Numpad3,
        KEY_NUMPAD4 => keyboard_types::Code::Numpad4,
        KEY_NUMPAD5 => keyboard_types::Code::Numpad5,
        KEY_NUMPAD6 => keyboard_types::Code::Numpad6,
        KEY_NUMPAD7 => keyboard_types::Code::Numpad7,
        KEY_NUMPAD8 => keyboard_types::Code::Numpad8,
        KEY_NUMPAD9 => keyboard_types::Code::Numpad9,
        KEY_MULTIPLY => keyboard_types::Code::NumpadMultiply,
        KEY_ADD => keyboard_types::Code::NumpadAdd,
        KEY_SUBTRACT => keyboard_types::Code::NumpadSubtract,
        KEY_DECIMAL => keyboard_types::Code::NumpadComma,
        KEY_DIVIDE => keyboard_types::Code::NumpadDivide,
        KEY_F1 => keyboard_types::Code::F1,
        KEY_F2 => keyboard_types::Code::F2,
        KEY_F3 => keyboard_types::Code::F3,
        KEY_F4 => keyboard_types::Code::F4,
        KEY_F5 => keyboard_types::Code::F5,
        KEY_F6 => keyboard_types::Code::F6,
        KEY_F7 => keyboard_types::Code::F7,
        KEY_F8 => keyboard_types::Code::F8,
        KEY_F9 => keyboard_types::Code::F9,
        KEY_F10 => keyboard_types::Code::F10,
        KEY_F11 => keyboard_types::Code::F11,
        KEY_F12 => keyboard_types::Code::F12,
        KEY_EQUALS => keyboard_types::Code::Equal,
        KEY_CONTEXTMENU => keyboard_types::Code::ContextMenu,
        _ => keyboard_types::Code::Unidentified,
    }
}
