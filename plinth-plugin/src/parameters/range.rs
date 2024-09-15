use crate::parameters::ParameterValue;

pub trait ParameterRange<T>: Send + Sync {
    fn clamp(&self, value: T) -> T;
    fn steps(&self) -> usize;
    fn plain_to_normalized(&self, plain: T) -> Option<ParameterValue>;
    fn normalized_to_plain(&self, normalized: ParameterValue) -> T;
}
