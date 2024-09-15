use std::ffi::c_void;

use raw_window_handle::RawWindowHandle;

#[cfg(target_os="linux")]
pub fn from_ptr(parent: *mut c_void) -> RawWindowHandle {
    let raw_window_handle = raw_window_handle::XlibWindowHandle::new(parent as _);
    RawWindowHandle::Xlib(raw_window_handle)
}

#[cfg(target_os="macos")]
pub fn from_ptr(parent: *mut c_void) -> RawWindowHandle {
    use std::ptr::NonNull;

    let raw_window_handle = raw_window_handle::AppKitWindowHandle::new(
        NonNull::new(parent as _).unwrap()
    );
    RawWindowHandle::AppKit(raw_window_handle)
}

#[cfg(target_os="windows")]
pub fn from_ptr(parent: *mut c_void) -> RawWindowHandle {
    let raw_window_handle = raw_window_handle::Win32WindowHandle::new(std::num::NonZeroIsize::new(parent as _).unwrap());
    RawWindowHandle::Win32(raw_window_handle)
}
