use std::{any::Any, fmt::Display, sync::{atomic::AtomicBool, Arc}};

use portable_atomic::{AtomicF64, Ordering};

use crate::{Parameter, ParameterFormatter, ParameterId, ParameterValue};

use super::{error::Error, info::ParameterInfo, parameter::ParameterPlain, ModulationChangedCallback};

const DEFAULT_FALSE_STRING: &str = "False";
const DEFAULT_TRUE_STRING: &str = "True";

pub type ValueChangedCallback = Arc<dyn Fn(ParameterId, bool) + Send + Sync>;

pub struct BoolParameter {
    info: ParameterInfo,

    value: AtomicBool,
    normalized_modulation: AtomicF64,

    formatter: Arc<dyn ParameterFormatter<bool>>,

    value_changed: Option<ValueChangedCallback>,
    modulation_changed: Option<ModulationChangedCallback>,
}

impl BoolParameter {
    pub fn new(id: impl Into<ParameterId>, name: impl Into<String>) -> Self {
        let info = ParameterInfo::new(id.into(), name.into())
            .with_steps(1);

        Self {
            info,
            value: false.into(),
            normalized_modulation: 0.0.into(),
            formatter: Arc::new(BoolFormatter::new(DEFAULT_FALSE_STRING, DEFAULT_TRUE_STRING)),
            value_changed: None,
            modulation_changed: None,
        }
    }

    pub fn with_path(mut self, path: String) -> Self {
        self.info = self.info.with_path(path);
        self
    }

    pub fn with_default_value(mut self, default_value: bool) -> Self {
        let default_normalized_value = if default_value { 1.0 } else { 0.0 };

        self.info = self.info.with_default_normalized_value(default_normalized_value);
        self.value.store(default_value, Ordering::Release);
        self
    }

    pub fn with_formatter(mut self, formatter: Arc<dyn ParameterFormatter<bool>>) -> Self {
        self.formatter = formatter;
        self
    }

    pub fn on_value_changed(mut self, value_changed: ValueChangedCallback) -> Self {
        self.value_changed = Some(value_changed);
        self
    }

    pub fn on_modulation_changed(mut self, modulation_changed: ModulationChangedCallback) -> Self {
        self.modulation_changed = Some(modulation_changed);
        self
    }

    pub fn as_bypass(mut self) -> Self {
        self.info = self.info.as_bypass();
        self
    }

    pub fn set_value(&self, value: bool) {
        self.value.store(value, Ordering::Release);

        if let Some(on_value_changed) = self.value_changed.as_ref() {
            on_value_changed(self.info.id(), self.plain());
        }
    }

    pub fn default_value(&self) -> bool {
        self.normalized_to_plain(self.info.default_normalized_value())
    }
}

impl Clone for BoolParameter {
    fn clone(&self) -> Self {
        Self {
            info: self.info.clone(),

            value: self.value.load(Ordering::Acquire).into(),
            normalized_modulation: self.normalized_modulation.load(Ordering::Acquire).into(),

            formatter: self.formatter.clone(),

            value_changed: self.value_changed.clone(),
            modulation_changed: self.modulation_changed.clone(),
        }
    }
}

impl Display for BoolParameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.formatter.value_to_string(self.plain()))
    }
}

impl Parameter for BoolParameter {
    fn info(&self) -> &ParameterInfo {
        &self.info
    }

    fn normalized_value(&self) -> ParameterValue {
        self.plain_to_normalized(self.value.load(Ordering::Acquire))
    }

    fn set_normalized_value(&self, normalized: ParameterValue) -> Result<(), Error> {
        let normalized = f64::clamp(normalized, 0.0, 1.0);
        self.set_value(self.normalized_to_plain(normalized));
        Ok(())
    }

    fn normalized_modulation(&self) -> ParameterValue {
        self.normalized_modulation.load(Ordering::Acquire)
    }

    fn set_normalized_modulation(&self, amount: ParameterValue) {
        self.normalized_modulation.store(amount, Ordering::Release);

        if let Some(on_modulated_value_changed) = self.modulation_changed.as_ref() {
            on_modulated_value_changed(self.info.id(), self.normalized_modulation());
        }
    }

    fn normalized_to_string(&self, value: ParameterValue) -> String {
        let value = self.normalized_to_plain(value);
        self.formatter.value_to_string(value)
    }

    fn string_to_normalized(&self, string: &str) -> Option<ParameterValue> {
        let plain = self.formatter.string_to_value(string)?;
        Some(self.plain_to_normalized(plain))
    }

    fn serialize_value(&self) -> ParameterValue {
        self.normalized_value()
    }

    fn deserialize_value(&self, value: ParameterValue) -> Result<(), Error> {
        self.set_normalized_value(value)
    }
    
    fn as_any(&self) -> &dyn Any {
        self as _
    }
}

impl ParameterPlain for BoolParameter {
    type Plain = bool;
    
    fn normalized_to_plain(&self, value: ParameterValue) -> bool {
        value >= 0.5
    }

    fn plain_to_normalized(&self, plain: bool) -> ParameterValue {
        if plain {
            1.0
        } else {
            0.0
        }
    }
}

pub struct BoolFormatter {
    false_string: &'static str,
    true_string: &'static str,
}

impl BoolFormatter {
    pub const fn new(false_string: &'static str, true_string: &'static str) -> Self {
        Self {
            false_string,
            true_string,
        }
    }
}

impl ParameterFormatter<bool> for BoolFormatter {
    fn value_to_string(&self, value: bool) -> String {
        if value {
            self.true_string.to_string()
        } else {
            self.false_string.to_string()
        }
    }

    fn string_to_value(&self, string: &str) -> Option<bool> {
        let string = string.to_lowercase();
        
        if string == self.false_string.to_lowercase() {
            Some(false)
        } else if string == self.true_string.to_lowercase() {
            Some(true)
        } else {
            None
        }
    }
}
