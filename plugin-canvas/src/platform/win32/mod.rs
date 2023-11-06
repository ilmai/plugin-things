use std::{ffi::OsString, str::FromStr, os::windows::prelude::OsStrExt};

use lazy_static::lazy_static;
use windows::Win32::{System::SystemServices::IMAGE_DOS_HEADER, Foundation::HINSTANCE, UI::WindowsAndMessaging::WM_USER};

pub mod cursors;
pub mod drop_target;
pub mod key_codes;
pub mod message_hook;
pub mod message_window;
pub mod version;
pub mod window;

extern "C" {
    static __ImageBase: IMAGE_DOS_HEADER;
}

lazy_static! {
    static ref PLUGIN_HINSTANCE: HINSTANCE = unsafe { HINSTANCE(&__ImageBase as *const IMAGE_DOS_HEADER as _) };
}

const WM_USER_FRAME_TIMER: u32  = WM_USER + 1;
const WM_USER_KEY_DOWN: u32     = WM_USER + 2;
const WM_USER_KEY_UP: u32       = WM_USER + 3;

fn to_wstr(string: impl AsRef<str>) -> Vec<u16> {
    let mut wstr: Vec<_> = OsString::from_str(string.as_ref()).unwrap().encode_wide().collect();
    wstr.push(0);
    wstr
}
