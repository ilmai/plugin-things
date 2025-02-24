use crate::parameters::info::ParameterInfo;

pub fn parameter_multiplier(info: &ParameterInfo) -> f64 {
    if info.steps() > 0 {
        info.steps() as f64
    } else {
        1.0
    }
}
