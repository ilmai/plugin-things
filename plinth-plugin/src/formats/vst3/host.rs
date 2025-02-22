use std::{cell::RefCell, rc::Rc};

use vst3::{ComPtr, Steinberg::Vst::{IComponentHandler, IComponentHandler2, IComponentHandler2Trait, IComponentHandlerTrait}};

use crate::{host::Host, parameters::ParameterValue, ParameterId, Parameters, Plugin};

pub struct Vst3Host<P: Plugin> {
    plugin: Rc<RefCell<P>>,
    handler: ComPtr<IComponentHandler>,
    name: Option<String>,
}

impl<P: Plugin> Vst3Host<P> {
    pub fn new(
        plugin: Rc<RefCell<P>>,
        handler: ComPtr<IComponentHandler>,
        name: Option<String>,
    ) -> Self {
        Self {
            plugin,
            handler,
            name,
        }
    }
}

impl<P: Plugin> Host for Vst3Host<P> {
    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn start_parameter_change(&self, id: ParameterId) {
        unsafe { self.handler.beginEdit(id) };
    }

    fn change_parameter_value(&self, id: ParameterId, normalized: ParameterValue) {
        self.plugin.borrow().with_parameters(|parameters| {
            let parameter = parameters.get(id).unwrap();
            parameter.set_normalized_value(normalized).unwrap();
        });

        unsafe { self.handler.performEdit(id, normalized) };
    }

    fn end_parameter_change(&self, id: ParameterId) {
        unsafe { self.handler.endEdit(id) };
    }
    
    fn mark_state_dirty(&self) {
        let handler2: ComPtr<IComponentHandler2> = self.handler.cast().unwrap();
        unsafe { handler2.setDirty(1) };
    }
}
