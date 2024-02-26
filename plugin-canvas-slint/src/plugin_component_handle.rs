use plugin_canvas::{Event, event::EventResponse};

pub trait PluginComponentHandle {
    fn window(&self) -> &slint::Window;
    fn on_event(&self, event: &Event) -> EventResponse;
}
