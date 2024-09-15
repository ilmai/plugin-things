pub mod bool;
pub mod enums;
pub mod error;
pub mod float;
pub mod formatter;
pub mod group;
pub mod info;
pub mod int;
pub mod kind;
pub mod map;
pub mod parameter;
pub mod parameters;
pub mod range;

pub use error::Error;
pub type ParameterId = u32;
pub type ParameterValue = f64;
