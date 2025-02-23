use crate::{parameters::ParameterValue, ParameterId};

pub trait Host {
    fn name(&self) -> Option<&str>;

    fn can_resize(&self) -> bool;

    /// Return true if the resize was accepted
    fn resize_view(&self, width: f64, height: f64) -> bool;

    fn start_parameter_change(&self, id: ParameterId);
    fn change_parameter_value(&self, id: ParameterId, normalized: ParameterValue);
    fn end_parameter_change(&self, id: ParameterId);

    fn mark_state_dirty(&self);
}
