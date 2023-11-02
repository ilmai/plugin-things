use std::any::Any;

use plugin_canvas::{Event, event::EventResponse};

pub trait PluginComponentHandle {
    fn as_any(&self) -> &dyn Any;
    fn window(&self) -> &slint::Window;

    fn on_event(&self, event: &Event) -> EventResponse;

    fn update_parameter(&self, id: &str, update_value: bool, update_modulation: bool);
    fn update_all_parameters(&self);
}
