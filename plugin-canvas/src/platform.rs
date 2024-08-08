pub mod interface;

#[cfg(target_os="linux")]
pub mod x11;
#[cfg(target_os="macos")]
pub mod mac;
#[cfg(target_os="windows")]
pub mod win32;

#[cfg(target_os="linux")]
pub use x11::*;
#[cfg(target_os="macos")]
pub use mac::*;
#[cfg(target_os="windows")]
pub use win32::*;
