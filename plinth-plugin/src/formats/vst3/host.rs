use vst3::{ComPtr, Steinberg::Vst::{IComponentHandler, IComponentHandler2, IComponentHandler2Trait, IComponentHandlerTrait}};

use crate::{host::Host, parameters::ParameterValue, ParameterId};

pub struct Vst3Host {
    handler: ComPtr<IComponentHandler>,
}

impl Vst3Host {
    pub fn new(handler: ComPtr<IComponentHandler>) -> Self {
        Self {
            handler,
        }
    }
}

impl Host for Vst3Host {
    fn start_parameter_change(&self, id: ParameterId) {
        unsafe { self.handler.beginEdit(id) };
    }

    fn change_parameter_value(&self, id: ParameterId, normalized: ParameterValue) {
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

// SAFETY: Technically calling IComponentHandler functions from another thread isn't up to the VST3 spec,
//         but on Linux the UI event loop lives in a separate thread from the DAW UI thread. We could use
//         VST3's IRunLoop to work around this, but it's not possible to use that unless a GUI is open so
//         we can't be always correct anyway. Deal with this if it becomes a problem.
unsafe impl Send for Vst3Host {}
