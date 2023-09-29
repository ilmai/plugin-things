use windows::{Win32::{UI::WindowsAndMessaging::{HCURSOR, LoadImageW, IDC_ARROW, IDC_HAND, IMAGE_CURSOR, LR_SHARED}, Foundation::HINSTANCE}, core::PCWSTR};

pub struct Cursors {
    pub arrow: HCURSOR,
    pub hand: HCURSOR,
}

impl Cursors {
    pub fn new() -> Self {
        Self {
            arrow: Self::load_cursor(IDC_ARROW),
            hand: Self::load_cursor(IDC_HAND),
        }
    }

    fn load_cursor(name: PCWSTR) -> HCURSOR {
        let handle = unsafe { LoadImageW(
            HINSTANCE(0),
            name,
            IMAGE_CURSOR,
            0,
            0,
            LR_SHARED,
        ).unwrap() };

        HCURSOR(handle.0)
    }
}

impl Default for Cursors {
    fn default() -> Self {
        Self::new()
    }
}
