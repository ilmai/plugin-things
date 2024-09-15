use crate::Parameter;

pub fn parameter_multiplier(parameter: &dyn Parameter) -> f64 {
    if parameter.info().steps() > 0 {
        parameter.info().steps() as f64
    } else {
        1.0
    }
}
