use crate::{parameters::ParameterValue, ParameterId};

pub trait Host {
    fn start_parameter_change(&self, id: ParameterId);
    fn change_parameter_value(&self, id: ParameterId, normalized: ParameterValue);
    fn end_parameter_change(&self, id: ParameterId);

    fn mark_state_dirty(&self);
}
