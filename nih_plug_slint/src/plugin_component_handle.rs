use std::any::Any;

use plugin_canvas::{Event, event::EventResponse};

pub trait PluginComponentHandle {
    fn as_any(&self) -> &dyn Any;
    fn window(&self) -> &slint::Window;

    fn on_event(&self, event: &Event) -> EventResponse;

    fn update_parameter(&self, id: &str, update_value: bool, update_modulation: bool);
    fn update_all_parameters(&self);
}

// fn update_parameter(&self, id: &str, update_value: bool, update_modulation: bool) {
//     let context = self.context.borrow();
//     let context = context.as_ref().unwrap();

//     // if let Some(param_ptr) = context.param_map.get(id) {
//     //     if let Ok(Value::Struct(mut plugin_parameter)) = context.component.get_global_property(&context.parameter_globals_name, &id) {
//     //         let value = unsafe { param_ptr.unmodulated_normalized_value() };
//     //         let modulation = unsafe { param_ptr.modulated_normalized_value() - value };

//     //         if update_value {
//     //             let display_value = unsafe { param_ptr.normalized_value_to_string(value, true) };

//     //             plugin_parameter.set_field("value".into(), Value::Number(value as f64));
//     //             plugin_parameter.set_field("display-value".into(), Value::String(display_value.into()));    
//     //             plugin_parameter.set_field("modulation".into(), Value::Number(modulation as f64));
//     //         } else if update_modulation {
//     //             plugin_parameter.set_field("modulation".into(), Value::Number(modulation as f64));
//     //         }

//     //         context.component.set_global_property(&context.parameter_globals_name, id, Value::Struct(plugin_parameter)).unwrap();
//     //     }
//     // }
// }

// fn update_all_parameters(&self) {
//     for id in self.ui_parameters.borrow().iter() {
//         self.update_parameter(id, true, true);
//     }
// }
