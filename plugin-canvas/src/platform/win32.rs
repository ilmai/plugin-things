use std::{ffi::OsString, str::FromStr, os::windows::prelude::OsStrExt};

use windows::Win32::{Foundation::HINSTANCE, System::SystemServices::IMAGE_DOS_HEADER, UI::WindowsAndMessaging::WM_APP};

pub mod cursors;
pub mod drop_target;
pub mod keyboard;
pub mod message_window;
pub mod version;
pub mod window;

unsafe extern "C" {
    static __ImageBase: IMAGE_DOS_HEADER;
}

thread_local! {
    static PLUGIN_HINSTANCE: HINSTANCE = unsafe { HINSTANCE(&__ImageBase as *const IMAGE_DOS_HEADER as _) };
}

const WM_APP_FRAME_TIMER: u32  = WM_APP;
const WM_APP_CHAR: u32         = WM_APP + 1;
const WM_APP_KEY_DOWN: u32     = WM_APP + 2;
const WM_APP_KEY_UP: u32       = WM_APP + 3;

fn to_wstr(string: impl AsRef<str>) -> Vec<u16> {
    let mut wstr: Vec<_> = OsString::from_str(string.as_ref()).unwrap().encode_wide().collect();
    wstr.push(0);
    wstr
}
