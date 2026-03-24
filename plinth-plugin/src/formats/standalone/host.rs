use std::sync::{Arc, mpsc::Sender};

use crate::{Event, Host, ParameterId, ParameterValue};

use super::parameters::StandaloneParameterEventMap;

pub struct StandaloneHost {
    parameter_event_map: Arc<StandaloneParameterEventMap>,
    to_plugin_sender: Sender<Event>,
}

impl StandaloneHost {
    pub fn new(
        parameter_event_map: Arc<StandaloneParameterEventMap>,
        to_plugin_sender: Sender<Event>,
    ) -> Self {
        Self {
            parameter_event_map,
            to_plugin_sender,
        }
    }
}

impl Host for StandaloneHost {
    fn can_resize(&self) -> bool {
        false
    }

    fn resize_view(&self, _width: f64, _height: f64) -> bool {
        false
    }

    fn start_parameter_change(&self, id: ParameterId) {
        let _ = self.to_plugin_sender.send(Event::StartParameterChange { id });
    }

    fn change_parameter_value(&self, id: ParameterId, normalized: ParameterValue) {
        self.parameter_event_map.change_parameter_value(id, normalized);

        let _ = self.to_plugin_sender.send(Event::ParameterValue {
            sample_offset: 0,
            id,
            value: normalized,
        });
    }

    fn end_parameter_change(&self, id: ParameterId) {
        let _ = self.to_plugin_sender.send(Event::EndParameterChange { id });
    }

    fn reload_parameters(&self) {}

    fn mark_state_dirty(&self) {}
}
