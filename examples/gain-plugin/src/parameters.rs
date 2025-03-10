use std::sync::Arc;

use plinth_derive::ParameterKind;
use plinth_plugin::{FloatFormatter, FloatParameter, LinearFloatRange, Parameter, ParameterId, ParameterMap, Parameters};

const MIN_GAIN: f64 = -80.0;
const MAX_GAIN: f64 = 80.0;

#[derive(ParameterKind)]
pub enum GainParameter {
    Gain,
}

#[derive(Clone)]
pub struct GainParameters {
    map: ParameterMap,
}

impl Default for GainParameters {
    fn default() -> Self {
        let mut map = ParameterMap::new();
        
        map.add(
            FloatParameter::new(
                GainParameter::Gain,
                "Gain",
                Arc::new(LinearFloatRange::new(MIN_GAIN, MAX_GAIN)),
            )
            .with_default_value(0.0)
            .with_formatter(Arc::new(FloatFormatter::new(1, "dB")))
        );

        Self {
            map,
        }
    }
}

impl Parameters for GainParameters {
    fn ids(&self) -> &[ParameterId] {
        self.map.ids()
    }

    fn get(&self, id: impl Into<ParameterId>) -> Option<&dyn Parameter> {
        self.map.get(id)
    }
}
