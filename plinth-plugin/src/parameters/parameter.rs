use std::any::Any;

use crate::error::Error;

use super::{info::ParameterInfo, ParameterValue};

pub trait Parameter : Any + Send + Sync {
    fn info(&self) -> &ParameterInfo;

    fn normalized_value(&self) -> ParameterValue;
    fn set_normalized_value(&self, normalized: ParameterValue) -> Result<(), Error> ;

    fn normalized_modulation(&self) -> ParameterValue;
    fn set_normalized_modulation(&self, amount: ParameterValue);

    fn normalized_to_string(&self, value: ParameterValue) -> String;
    fn string_to_normalized(&self, string: &str) -> Option<ParameterValue>;

    fn serialize_value(&self) -> ParameterValue;
    fn deserialize_value(&self, value: ParameterValue) -> Result<(), Error> ;
}

pub trait ParameterPlain : Parameter {
    type Plain;

    fn normalized_to_plain(&self, normalized: ParameterValue) -> Self::Plain;
    fn plain_to_normalized(&self, plain: Self::Plain) -> ParameterValue;

    fn plain(&self) -> Self::Plain {
        self.normalized_to_plain(self.normalized_value())
    }

    fn modulated_plain(&self) -> Self::Plain {
        self.normalized_to_plain(self.normalized_value() + self.normalized_modulation())
    }
}
