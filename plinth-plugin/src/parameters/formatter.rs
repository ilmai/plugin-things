pub trait ParameterFormatter<T> : Send + Sync {
    fn value_to_string(&self, value: T) -> String;
    fn string_to_value(&self, string: &str) -> Option<T>;
}
