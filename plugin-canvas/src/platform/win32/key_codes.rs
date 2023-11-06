use windows::Win32::UI::Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_RETURN, VK_SHIFT, VK_CONTROL, VK_MENU, VK_DELETE, VK_UP, VK_DOWN, VK_LEFT, VK_RIGHT};

pub(super) fn virtual_key_to_char(key: VIRTUAL_KEY) -> Option<usize> {
    match key {
        VK_RETURN   => Some(0x000a),
        VK_SHIFT    => Some(0x0010),
        VK_CONTROL  => Some(0x0011),
        VK_MENU     => Some(0x0012),
        VK_DELETE   => Some(0x007f),
        VK_UP       => Some(0xf700),
        VK_DOWN     => Some(0xf701),
        VK_LEFT     => Some(0xf702),
        VK_RIGHT    => Some(0xf703),
        _           => None,
    }
}
