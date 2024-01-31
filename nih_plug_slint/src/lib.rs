pub mod plugin_component_handle;
pub mod editor;
pub mod platform;
pub mod window_adapter;

// Re-exports
pub use plugin_canvas::dimensions::{LogicalPosition, LogicalSize};
pub use plugin_canvas::drag_drop::{DropData, DropOperation};
pub use plugin_canvas::event::{Event, EventResponse};
pub use plugin_canvas::window::{Window, WindowAttributes};
