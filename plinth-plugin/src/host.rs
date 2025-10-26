use crate::ParameterId;
use crate::formats::PluginFormat;
use crate::parameters::ParameterValue;

#[derive(Clone)]
pub struct HostInfo {
    pub name: Option<String>,
    pub format: PluginFormat,
}

pub trait Host {
    fn can_resize(&self) -> bool;

    /// Return true if the resize was accepted
    fn resize_view(&self, width: f64, height: f64) -> bool;

    fn start_parameter_change(&self, id: ParameterId);
    fn change_parameter_value(&self, id: ParameterId, normalized: ParameterValue);
    fn end_parameter_change(&self, id: ParameterId);

    /// Call when all/most parameters have changed, for example when loading a preset
    fn reload_parameters(&self);

    fn mark_state_dirty(&self);
}
