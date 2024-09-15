use std::sync::mpsc;

use clap_sys::{ext::{params::clap_host_params, state::clap_host_state}, host::clap_host};

use crate::{Event, Host, ParameterId, ParameterValue};

pub struct ClapHost {
    raw: *const clap_host,
    host_ext_params: *const clap_host_params,
    host_ext_state: *const clap_host_state,
    event_sender: mpsc::Sender<Event>,
}

impl ClapHost {
    pub fn new(
        raw: *const clap_host,
        host_ext_params: *const clap_host_params,
        host_ext_state: *const clap_host_state,
        event_sender: mpsc::Sender<Event>,
    ) -> Self {
        assert!(!raw.is_null());

        Self {
            raw,
            host_ext_params,
            host_ext_state,
            event_sender,
        }
    }
}

impl Host for ClapHost {
    fn start_parameter_change(&self, id: ParameterId) {
        self.event_sender.send(Event::StartParameterChange { id }).unwrap();
        
        if !self.host_ext_params.is_null() {
            unsafe { ((*self.host_ext_params).request_flush.unwrap())(self.raw) };
        }
    }

    fn change_parameter_value(&self, id: ParameterId, normalized: ParameterValue) {
        self.event_sender.send(Event::ParameterValue { sample_offset: 0, id, value: normalized }).unwrap();

        if !self.host_ext_params.is_null() {
            unsafe { ((*self.host_ext_params).request_flush.unwrap())(self.raw) };
        }
    }

    fn end_parameter_change(&self, id: ParameterId) {
        self.event_sender.send(Event::EndParameterChange { id }).unwrap();

        if !self.host_ext_params.is_null() {
            unsafe { ((*self.host_ext_params).request_flush.unwrap())(self.raw) };
        }
    }
    
    fn mark_state_dirty(&self) {
        if !self.host_ext_state.is_null() {
            unsafe { ((*self.host_ext_state).mark_dirty.unwrap())(self.raw) };
        }
    }
}

/// SAFETY: clap_host functions are thread-safe
unsafe impl Send for ClapHost {}
