use windows::{Win32::{UI::WindowsAndMessaging::{HCURSOR, LoadImageW, IDC_ARROW, IDC_HAND, IMAGE_CURSOR, LR_SHARED, IDC_APPSTARTING, IDC_CROSS, IDC_HELP, IDC_IBEAM, IDC_SIZEALL, IDC_NO, IDC_SIZEWE, IDC_SIZENESW, IDC_SIZENWSE, IDC_WAIT, IDC_SIZENS}, Foundation::HINSTANCE}, core::PCWSTR};

pub struct Cursors {
    pub app_starting: HCURSOR,
    pub arrow: HCURSOR,
    pub cross: HCURSOR,
    pub hand: HCURSOR,
    pub help: HCURSOR,
    pub ibeam: HCURSOR,
    pub no: HCURSOR,
    pub size_all: HCURSOR,
    pub size_ns: HCURSOR,
    pub size_ew: HCURSOR,
    pub size_nesw: HCURSOR,
    pub size_nwse: HCURSOR,    
    pub wait: HCURSOR,
}

impl Cursors {
    pub fn new() -> Self {
        Self {
            app_starting: Self::load_cursor(IDC_APPSTARTING),
            arrow: Self::load_cursor(IDC_ARROW),
            cross: Self::load_cursor(IDC_CROSS),
            hand: Self::load_cursor(IDC_HAND),
            help: Self::load_cursor(IDC_HELP),
            ibeam: Self::load_cursor(IDC_IBEAM),
            size_all: Self::load_cursor(IDC_SIZEALL),
            no: Self::load_cursor(IDC_NO),
            size_ns: Self::load_cursor(IDC_SIZENS),
            size_ew: Self::load_cursor(IDC_SIZEWE),
            size_nesw: Self::load_cursor(IDC_SIZENESW),
            size_nwse: Self::load_cursor(IDC_SIZENWSE),
            wait: Self::load_cursor(IDC_WAIT),
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
