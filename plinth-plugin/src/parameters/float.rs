use std::{any::Any, fmt::Display, sync::Arc};

use portable_atomic::{AtomicF64, Ordering};

use crate::{Parameter, ParameterId};

use super::{error::Error, formatter::ParameterFormatter, info::ParameterInfo, parameter::ParameterPlain, range::ParameterRange, ParameterValue};

pub const DEFAULT_PRECISION: usize = 2;

pub type ValueChangedCallback = Arc<dyn Fn(f64) + Send + Sync>;

pub struct FloatParameter {
    info: ParameterInfo,
    value: AtomicF64,
    normalized_modulation: AtomicF64,
    range: Arc<dyn ParameterRange<f64>>,
    formatter: Arc<dyn ParameterFormatter<f64>>,
    value_changed: Option<ValueChangedCallback>,
}

impl FloatParameter {
    pub fn new(id: impl Into<ParameterId>, name: impl Into<String>, range: Arc<dyn ParameterRange<f64>>) -> Self {
        let value = range.normalized_to_plain(0.0);

        Self {
            info: ParameterInfo::new(id.into(), name.into()),
            value: value.into(),
            normalized_modulation: 0.0.into(),
            range,
            formatter: Arc::new(FloatFormatter::new(DEFAULT_PRECISION, "")),
            value_changed: None,
        }
    }

    pub fn with_path(mut self, path: String) -> Self {
        self.info = self.info.with_path(path);
        self
    }

    pub fn with_default_value(mut self, value: f64) -> Self {
        let default_normalized_value = self.range.plain_to_normalized(value).unwrap();
        self.info = self.info.with_default_normalized_value(default_normalized_value);
        self.value.store(value, Ordering::Release);
        self
    }

    pub fn with_formatter(mut self, formatter: Arc<dyn ParameterFormatter<f64>>) -> Self {
        self.formatter = formatter;
        self
    }

    pub fn on_value_changed(mut self, value_changed: ValueChangedCallback) -> Self {
        self.value_changed = Some(value_changed);
        self
    }

    pub fn set_value(&self, value: f64) {
        self.value.store(value, Ordering::Release);
        self.changed();
    }

    pub fn default_value(&self) -> f64 {
        self.range.normalized_to_plain(self.info.default_normalized_value())
    }

    fn changed(&self) {
        if let Some(on_value_changed) = self.value_changed.as_ref() {
            on_value_changed(self.plain());
        }
    }
}

impl Clone for FloatParameter {
    fn clone(&self) -> Self {
        Self {
            info: self.info.clone(),
            value: self.value.load(Ordering::Acquire).into(),
            normalized_modulation: self.normalized_modulation.load(Ordering::Acquire).into(),
            range: self.range.clone(),
            formatter: self.formatter.clone(),
            value_changed: self.value_changed.clone(),
        }
    }
}

impl Display for FloatParameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.formatter.value_to_string(self.plain()))
    }
}

impl Parameter for FloatParameter {
    fn info(&self) -> &ParameterInfo {
        &self.info
    }

    fn normalized_value(&self) -> ParameterValue {
        self.range.plain_to_normalized(self.value.load(Ordering::Acquire)).unwrap()
    }

    fn set_normalized_value(&self, normalized: ParameterValue) -> Result<(), Error> {
        let normalized = f64::clamp(normalized, 0.0, 1.0);
        self.set_value(self.range.normalized_to_plain(normalized));
        Ok(())
    }

    fn normalized_modulation(&self) -> ParameterValue {
        self.normalized_modulation.load(Ordering::Acquire)
    }

    fn set_normalized_modulation(&self, amount: ParameterValue) {
        self.normalized_modulation.store(amount, Ordering::Release);
        self.changed();
    }

    fn normalized_to_string(&self, normalized: ParameterValue) -> String {
        let plain = self.range.normalized_to_plain(normalized);
        self.formatter.value_to_string(plain)
    }

    fn string_to_normalized(&self, string: &str) -> Option<ParameterValue> {
        let plain = self.formatter.string_to_value(string)?;        
        self.range.plain_to_normalized(plain)
    }

    fn serialize_value(&self) -> ParameterValue {
        self.value.load(Ordering::Acquire)
    }

    fn deserialize_value(&self, value: ParameterValue) -> Result<(), Error> {
        let value = self.range.clamp(value);
        self.set_value(value);
        Ok(())
    }
    
    fn as_any(&self) -> &dyn Any {
        self as _
    }
}

impl ParameterPlain for FloatParameter {
    type Plain = f64;
    
    fn normalized_to_plain(&self, normalized: ParameterValue) -> f64 {
        let normalized = normalized.clamp(0.0, 1.0);
        self.range.normalized_to_plain(normalized)
    }

    fn plain_to_normalized(&self, plain: f64) -> ParameterValue {
        self.range.plain_to_normalized(plain).unwrap()
    }
}

#[derive(Clone)]
pub struct LinearFloatRange {
    min: f64,
    max: f64,
}

impl LinearFloatRange {
    pub fn new(min: f64, max: f64) -> Self {
        assert!(min < max);
        
        Self {
            min,
            max,
        }
    }
}

impl ParameterRange<f64> for LinearFloatRange {
    fn clamp(&self, value: f64) -> f64 {
        f64::clamp(value, self.min, self.max)
    }

    fn steps(&self) -> usize {
        0
    }

    fn plain_to_normalized(&self, plain: f64) -> Option<ParameterValue> {
        if plain < self.min || plain > self.max {
            return None;
        }

        Some((plain - self.min) / (self.max - self.min))
    }

    fn normalized_to_plain(&self, normalized: ParameterValue) -> f64 {
        normalized * (self.max - self.min) + self.min
    }
}

#[derive(Clone)]
pub struct LogFloatRange {
    min: f64,
    max: f64,
    k: f64,
}

impl LogFloatRange {
    pub fn new(min: f64, max: f64, k: f64) -> Self {
        assert!(k > 1.0);

        Self {
            min,
            max,
            k,
        }
    }    
}

impl ParameterRange<f64> for LogFloatRange {
    fn clamp(&self, value: f64) -> f64 {
        f64::clamp(value, self.min, self.max)
    }

    fn steps(&self) -> usize {
        0
    }

    fn plain_to_normalized(&self, plain: f64) -> Option<ParameterValue> {
        if plain < self.min || plain > self.max {
            return None;
        }

        let x = (plain - self.min) / (self.max - self.min);
        Some(f64::log(x * (self.k - 1.0) + 1.0, self.k))
    }

    fn normalized_to_plain(&self, normalized: ParameterValue) -> f64 {
        let x = (f64::powf(self.k, normalized) - 1.0) / (self.k - 1.0);
        x * (self.max - self.min) + self.min
    }
}

#[derive(Clone)]
pub struct PowFloatRange {
    min: f64,
    max: f64,
    k: f64,
}

impl PowFloatRange {
    pub fn new(min: f64, max: f64, k: f64) -> Self {
        assert!(k > 1.0);

        Self {
            min,
            max,
            k,
        }
    }    
}

impl ParameterRange<f64> for PowFloatRange {
    fn clamp(&self, value: f64) -> f64 {
        f64::clamp(value, self.min, self.max)
    }

    fn steps(&self) -> usize {
        0
    }

    fn plain_to_normalized(&self, plain: f64) -> Option<ParameterValue> {
        if plain < self.min || plain > self.max {
            return None;
        }

        let x = (plain - self.min) / (self.max - self.min);
        Some((f64::powf(self.k, x) - 1.0) / (self.k - 1.0))
    }

    fn normalized_to_plain(&self, normalized: ParameterValue) -> f64 {
        let x = f64::log(normalized * (self.k - 1.0) + 1.0, self.k);
        x * (self.max - self.min) + self.min
    }
}

pub struct FloatFormatter {
    precision: usize,
    unit: &'static str,
}

impl FloatFormatter {
    pub const fn new(precision: usize, unit: &'static str) -> Self {
        Self {
            precision,
            unit,
        }
    }
}

impl ParameterFormatter<f64> for FloatFormatter {
    fn value_to_string(&self, value: f64) -> String {
        // Never return "-0.0" or roundtrip conversion will fail and it will look weird anyway
        let precision = self.precision;
        let multiplier = usize::pow(10, precision as _) as f64;

        let value = if (value * multiplier).round() / multiplier == 0.0 {
            0.0
        } else {
            value
        };
    
        format!("{value:.precision$}{}", self.unit)
    }

    fn string_to_value(&self, string: &str) -> Option<f64> {
        let string = string.strip_suffix(self.unit).unwrap_or(string);
        string.parse().ok()
    }
}

pub struct HzFormatter {
    hz_precision: usize,
    khz_precision: usize,
}

impl HzFormatter {
    pub fn new(hz_precision: usize, khz_precision: usize) -> Self {
        Self {
            hz_precision,
            khz_precision,
        }
    }
}

impl ParameterFormatter<f64> for HzFormatter {
    fn value_to_string(&self, value: f64) -> String {
        let (value, precision, unit) = if value.round() < 1000.0 {
            (value, self.hz_precision, "Hz")
        } else {
            (value / 1000.0, self.khz_precision, "kHz")
        };
        
        format!("{value:.precision$}{}", unit, precision = precision)
    }

    fn string_to_value(&self, string: &str) -> Option<f64> {
        let string = string.to_lowercase();

        let (string, multiplier) = if string.ends_with("khz") {
            (string.strip_suffix("khz").unwrap(), 1000.0)
        } else {
            (string.strip_suffix("hz").unwrap_or(&string), 1.0)
        };

        string.parse()
            .map(|value: f64| value * multiplier)
            .ok()
    }
}

pub struct SecondsFormatter {
    s_precision: usize,
    ms_precision: usize,
}

impl SecondsFormatter {
    pub fn new(s_precision: usize, ms_precision: usize) -> Self {
        Self {
            s_precision,
            ms_precision,
        }
    }
}

impl ParameterFormatter<f64> for SecondsFormatter {
    fn value_to_string(&self, value: f64) -> String {
        let (value, precision, unit) = if (value * 1000.0).round() < 1000.0 {
            (value * 1000.0, self.ms_precision, "ms")
        } else {
            (value, self.s_precision, "s")
        };
        
        format!("{value:.precision$}{}", unit, precision = precision)
    }

    fn string_to_value(&self, string: &str) -> Option<f64> {
        let string = string.to_lowercase();

        let (string, multiplier) = if string.ends_with("ms") {
            (string.strip_suffix("ms").unwrap(), 0.001)
        } else {
            (string.strip_suffix("s").unwrap_or(&string), 1.0)
        };

        string.parse()
            .map(|value: f64| value * multiplier)
            .ok()
    }
}

pub struct PercentageFormatter {
    precision: usize,
}

impl PercentageFormatter {
    pub fn new(precision: usize) -> Self {
        Self {
            precision,
        }
    }
}
impl ParameterFormatter<f64> for PercentageFormatter {
    fn value_to_string(&self, value: f64) -> String {
        format!("{:.precision$}%", value * 100.0, precision = self.precision)
    }

    fn string_to_value(&self, string: &str) -> Option<f64> {
        let string = string.strip_suffix("%").unwrap_or(string);
        string.parse()
            .ok()
            .map(|value: f64| value / 100.0)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_ulps_eq;

    use crate::{parameters::range::ParameterRange, LogFloatRange, PowFloatRange};

    use super::LinearFloatRange;

    #[test]
    fn linear_float_converter() {
        let converter = LinearFloatRange::new(1.0, 3.0);
        assert_eq!(converter.plain_to_normalized(1.0), Some(0.0));
        assert_eq!(converter.plain_to_normalized(3.0), Some(1.0));
        assert!(converter.plain_to_normalized(0.0).is_none());
        assert!(converter.plain_to_normalized(4.0).is_none());
        assert_eq!(converter.normalized_to_plain(0.0), 1.0);
        assert_eq!(converter.normalized_to_plain(1.0), 3.0);
        assert_eq!(converter.plain_to_normalized(converter.normalized_to_plain(0.5)), Some(0.5));
    }

    #[test]
    fn log_float_converter() {
        let converter = LogFloatRange::new(1.0, 3.0, 2.0);
        assert_ulps_eq!(converter.plain_to_normalized(1.0).unwrap(), 0.0);
        assert_ulps_eq!(converter.plain_to_normalized(3.0).unwrap(), 1.0);
        assert!(converter.plain_to_normalized(0.0).is_none());
        assert!(converter.plain_to_normalized(4.0).is_none());
        assert_ulps_eq!(converter.normalized_to_plain(0.0), 1.0);
        assert_ulps_eq!(converter.normalized_to_plain(1.0), 3.0);
        assert_ulps_eq!(converter.plain_to_normalized(converter.normalized_to_plain(0.5)).unwrap(), 0.5);
    }

    #[test]
    fn pow_float_converter() {
        let converter = PowFloatRange::new(1.0, 3.0, 2.0);
        assert_ulps_eq!(converter.plain_to_normalized(1.0).unwrap(), 0.0);
        assert_ulps_eq!(converter.plain_to_normalized(3.0).unwrap(), 1.0);
        assert!(converter.plain_to_normalized(0.0).is_none());
        assert!(converter.plain_to_normalized(4.0).is_none());
        assert_ulps_eq!(converter.normalized_to_plain(0.0), 1.0);
        assert_ulps_eq!(converter.normalized_to_plain(1.0), 3.0);
        assert_ulps_eq!(converter.plain_to_normalized(converter.normalized_to_plain(0.5)).unwrap(), 0.5);
    }
}
