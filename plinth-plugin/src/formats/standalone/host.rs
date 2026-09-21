use std::{rc::Rc, sync::Arc};

use crate::{Host, ParameterId, ParameterValue, Parameters, Plugin};

use super::parameters::StandaloneParameterEventMap;

pub struct StandaloneHost<P: Plugin> {
    plugin: Rc<P>,
    parameter_event_map: Arc<StandaloneParameterEventMap>,
}

impl<P: Plugin> StandaloneHost<P> {
    pub fn new(plugin: Rc<P>, parameter_event_map: Arc<StandaloneParameterEventMap>) -> Self {
        Self {
            plugin,
            parameter_event_map,
        }
    }
}

impl<P: Plugin> Host for StandaloneHost<P> {
    fn can_resize(&self) -> bool {
        false
    }

    fn resize_view(&self, _width: f64, _height: f64) -> bool {
        false
    }

    fn change_parameter_value(&self, id: ParameterId, normalized: ParameterValue) {
        // Directly set the new value in the main thread.
        self.plugin.with_parameters(|parameters| {
            if let Some(parameter) = parameters.get(id) {
                parameter.set_normalized_value(normalized);
            } else {
                tracing::warn!("Unknown parameter: {id}");
            }
        });
        // Add parameter change event for the processor.
        self.parameter_event_map
            .change_parameter_value(id, normalized);
    }

    fn start_parameter_change(&self, _id: ParameterId) {}
    fn end_parameter_change(&self, _id: ParameterId) {}

    fn reload_parameters(&self) {}

    fn mark_state_dirty(&self) {}
}
