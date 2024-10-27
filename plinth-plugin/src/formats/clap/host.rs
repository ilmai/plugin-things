use std::sync::{atomic::Ordering, Arc};

use clap_sys::{ext::{params::clap_host_params, state::clap_host_state}, host::clap_host};

use crate::{Host, ParameterId, ParameterValue};

use super::parameters::ParameterEventMap;

pub struct ClapHost {
    raw: *const clap_host,
    host_ext_params: *const clap_host_params,
    host_ext_state: *const clap_host_state,
    parameter_event_map: Arc<ParameterEventMap>,
}

impl ClapHost {
    pub fn new(
        raw: *const clap_host,
        host_ext_params: *const clap_host_params,
        host_ext_state: *const clap_host_state,
        parameter_event_map: Arc<ParameterEventMap>,
    ) -> Self {
        assert!(!raw.is_null());

        Self {
            raw,
            host_ext_params,
            host_ext_state,
            parameter_event_map,
        }
    }
}

impl Host for ClapHost {
    fn start_parameter_change(&self, id: ParameterId) {
        self.parameter_event_map.parameter_event_info(id).change_started.store(true, Ordering::Release);
        
        if !self.host_ext_params.is_null() {
            unsafe { ((*self.host_ext_params).request_flush.unwrap())(self.raw) };
        }
    }

    fn change_parameter_value(&self, id: ParameterId, normalized: ParameterValue) {
        let parameter_event_info = self.parameter_event_map.parameter_event_info(id);

        parameter_event_info.value.store(normalized, Ordering::Release);
        parameter_event_info.changed.store(true, Ordering::Release);

        if !self.host_ext_params.is_null() {
            unsafe { ((*self.host_ext_params).request_flush.unwrap())(self.raw) };
        }
    }

    fn end_parameter_change(&self, id: ParameterId) {
        self.parameter_event_map.parameter_event_info(id).change_ended.store(true, Ordering::Release);

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
