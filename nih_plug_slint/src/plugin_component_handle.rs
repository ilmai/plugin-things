use std::{any::Any, collections::HashMap};

use nih_plug::prelude::ParamPtr;
use plugin_canvas::{Event, event::EventResponse};

pub trait PluginComponentHandle {
    fn as_any(&self) -> &dyn Any;
    fn window(&self) -> &slint::Window;
    fn param_map(&self) -> &HashMap<slint::SharedString, ParamPtr>;

    fn on_event(&self, event: &Event) -> EventResponse;

    fn update_parameter_value(&self, id: &str);
    fn update_parameter_modulation(&self, id: &str);
    fn update_all_parameters(&self);
}

pub trait PluginComponentHandleParameterEvents: PluginComponentHandle {
    fn on_start_parameter_change(&self, f: impl FnMut(slint::SharedString) + 'static);
    fn on_parameter_changed(&self, f: impl FnMut(slint::SharedString, f32) + 'static);
    fn on_end_parameter_change(&self, f: impl FnMut(slint::SharedString) + 'static);
    fn on_set_parameter_string(&self, f: impl FnMut(slint::SharedString, slint::SharedString) + 'static);
}
