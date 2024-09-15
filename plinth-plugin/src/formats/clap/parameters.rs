use crate::parameters::info::ParameterInfo;

pub fn map_parameter_value_to_clap(info: &ParameterInfo, value: f64) -> f64 {
    let steps = info.steps();
    if steps > 0 {
        (value * steps as f64).round()
    } else {
        value
    }
}

pub fn map_parameter_value_from_clap(info: &ParameterInfo, value: f64) -> f64 {
    let steps = info.steps();
    if steps > 0 {
        value / steps as f64
    } else {
        value
    }
}
