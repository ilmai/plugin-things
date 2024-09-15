use crate::{parameters::ParameterValue, ParameterId};

// SAFETY: These functions might be called from any thread so implementations need to be thread-safe
pub trait Host: Send {
    fn start_parameter_change(&self, id: ParameterId);
    fn change_parameter_value(&self, id: ParameterId, normalized: ParameterValue);
    fn end_parameter_change(&self, id: ParameterId);

    fn mark_state_dirty(&self);
}
