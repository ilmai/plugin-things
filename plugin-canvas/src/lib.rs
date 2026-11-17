pub mod dimensions;
pub mod drag_drop;
pub mod error;
pub mod event;
pub mod keyboard;
pub mod screen;
pub mod thread_bound;
pub mod window;

pub use dimensions::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};
pub use event::{Event, MouseButton};
pub use window::Window;
pub use window::WindowAttributes;

#[cfg(target_os="macos")]
pub use platform::mac::is_macos_version_at_least;

mod platform;
