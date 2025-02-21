use std::{any::Any, fmt::Display, marker::PhantomData, sync::{atomic::AtomicUsize, Arc}};

use portable_atomic::{AtomicF64, Ordering};

use crate::ParameterId;

use super::{error::Error, info::ParameterInfo, parameter::{Parameter, ParameterPlain}, range::ParameterRange, ParameterValue};

pub type ValueChangedCallback<T> = Arc<dyn Fn(T) + Send + Sync>;

pub trait Enum: Clone + Copy + Default + Send + Sync + 'static {
    const COUNT: usize;
    
    fn from_usize(value: usize) -> Option<Self>;
    fn from_string(string: &str) -> Option<Self>;    
    fn to_usize(&self) -> usize;
    fn to_string(&self) -> String;
}

pub struct EnumParameter<T: Enum> {
    info: ParameterInfo,
    value: AtomicUsize,
    normalized_modulation: AtomicF64,
    range: IntRange,
    value_changed: Option<ValueChangedCallback<T>>,

    _phantom_enum: PhantomData<T>,
}

impl<T: Enum> EnumParameter<T> {
    pub fn new(id: impl Into<ParameterId>, name: impl Into<String>) -> Self {
        assert!(T::COUNT > 0);

        let range = IntRange::new(0, T::COUNT as i64 - 1);
        let info = ParameterInfo::new(id.into(), name.into())
            .with_steps(T::COUNT - 1)
            .with_default_normalized_value(range.plain_to_normalized(T::default().to_usize() as i64).unwrap());

        Self {
            info,
            value: T::default().to_usize().into(),
            normalized_modulation: 0.0.into(),
            range,
            value_changed: None,

            _phantom_enum: PhantomData,
        }
    }

    pub fn with_path(mut self, path: String) -> Self {
        self.info = self.info.with_path(path);
        self
    }

    pub fn with_default_value(mut self, default_value: T) -> Self {
        let default_normalized_value = self.range.plain_to_normalized(default_value.to_usize() as i64).unwrap();

        self.info = self.info.with_default_normalized_value(default_normalized_value);
        self.value.store(default_value.to_usize(), Ordering::Release);
        self
    }

    pub fn on_value_changed(mut self, value_changed: ValueChangedCallback<T>) -> Self {
        self.value_changed = Some(value_changed);
        self
    }

    pub fn unmodulated_value(&self) -> T {
        T::from_usize(self.value.load(Ordering::Acquire)).unwrap()
    }

    pub fn set_value(&self, value: T) {
        self.value.store(value.to_usize(), Ordering::Release);
        self.changed();
    }

    pub fn default_value(&self) -> i64 {
        self.range.normalized_to_plain(self.info.default_normalized_value())
    }

    fn changed(&self) {
        if let Some(on_value_changed) = self.value_changed.as_ref() {
            on_value_changed(self.plain());
        }
    }
}

impl<T: Enum> Clone for EnumParameter<T> {
    fn clone(&self) -> Self {
        Self {
            info: self.info.clone(),
            value: self.value.load(Ordering::Acquire).into(),
            normalized_modulation: self.normalized_modulation.load(Ordering::Acquire).into(),
            range: self.range.clone(),
            value_changed: self.value_changed.clone(),

            _phantom_enum: PhantomData,
        }
    }
}

impl<T: Enum> Display for EnumParameter<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.plain().to_string())
    }
}

impl<T: Enum> Parameter for EnumParameter<T> {
    fn info(&self) -> &ParameterInfo {
        &self.info
    }

    fn normalized_value(&self) -> ParameterValue {
        self.range.plain_to_normalized(self.value.load(Ordering::Acquire) as i64).unwrap()
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
        self.changed();
    }

    fn normalized_to_string(&self, value: ParameterValue) -> String {
        self.normalized_to_plain(value).to_string()
    }

    fn string_to_normalized(&self, string: &str) -> Option<ParameterValue> {        
        let plain = T::from_string(string)?;        
        self.range.plain_to_normalized(plain.to_usize() as i64)
    }

    fn serialize_value(&self) -> ParameterValue {
        self.value.load(Ordering::Acquire) as _
    }

    fn deserialize_value(&self, value: ParameterValue) -> Result<(), Error> {
        if T::from_usize(value.round() as _).is_none() {
            return Err(Error::RangeError);
        }

        self.value.store(value.round() as usize, Ordering::Release);
        self.changed();

        Ok(())
    }
    
    fn as_any(&self) -> &dyn Any {
        self as _
    }
}

impl<T: Enum> ParameterPlain for EnumParameter<T> {
    type Plain = T;
    
    fn normalized_to_plain(&self, normalized: ParameterValue) -> T {
        let value = self.range.normalized_to_plain(normalized);
        let value = value.clamp(0, T::COUNT as i64);

        T::from_usize(value as usize).unwrap()
    }

    fn plain_to_normalized(&self, plain: T) -> ParameterValue {
        self.range.plain_to_normalized(plain.to_usize() as i64).unwrap()
    }
}

#[derive(Clone)]
pub struct IntRange {
    min: i64,
    max: i64,
}

impl IntRange {
    pub fn new(min: i64, max: i64) -> Self {
        assert_ne!(min, max);

        Self {
            min,
            max,
        }
    }
}

impl ParameterRange<i64> for IntRange {
    fn clamp(&self, value: i64) -> i64 {
        i64::clamp(value, self.min, self.max)
    }

    fn steps(&self) -> usize {
        i64::abs(self.max - self.min) as usize
    }

    fn plain_to_normalized(&self, plain: i64) -> Option<ParameterValue> {
        if plain < self.min || plain > self.max {
            return None;
        }

        Some((plain - self.min) as f64 / self.steps() as f64)
    }

    fn normalized_to_plain(&self, normalized: ParameterValue) -> i64 {
        let steps = self.steps();
        self.min + i64::min(steps as i64, (normalized * (steps + 1) as f64) as i64)
    }
}
