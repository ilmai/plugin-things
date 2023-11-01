use std::any::Any;

pub trait PluginComponentHandle {
    fn as_any(&self) -> &dyn Any;
    fn window(&self) -> &slint::Window;
}
