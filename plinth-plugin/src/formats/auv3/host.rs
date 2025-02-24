use std::{collections::HashMap, ffi::c_void, sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex}};

use crate::{Host, ParameterId, ParameterValue};

use super::{parameter_multiplier, parameters::CachedParameter};

pub struct Auv3Host {
    context: *mut c_void,
    start_parameter_change: unsafe extern "C-unwind" fn(*mut c_void, u32),
    change_parameter_value: unsafe extern "C-unwind" fn(*mut c_void, u32, f32),
    end_parameter_change: unsafe extern "C-unwind" fn(*mut c_void, u32),
    
    sending_parameter_change_from_editor: Arc<AtomicBool>,
    parameter_index_from_id: HashMap<ParameterId, usize>,
    cached_parameters: Arc<Mutex<Vec<CachedParameter>>>,
}

impl Auv3Host {
    pub(super) fn new(
        editor_context: *mut c_void,
        start_parameter_change: unsafe extern "C-unwind" fn(*mut c_void, u32),
        change_parameter_value: unsafe extern "C-unwind" fn(*mut c_void, u32, f32),
        end_parameter_change: unsafe extern "C-unwind" fn(*mut c_void, u32),
        sending_parameter_change_from_editor: Arc<AtomicBool>,
        cached_parameters: Arc<Mutex<Vec<CachedParameter>>>,
        parameter_index_from_id: HashMap<ParameterId, usize>,
    ) -> Self
    {
        Self {
            context: editor_context,
            start_parameter_change,
            end_parameter_change,
            change_parameter_value,

            sending_parameter_change_from_editor,
            parameter_index_from_id,
            cached_parameters,
        }
    }
}

impl Host for Auv3Host {
    fn name(&self) -> Option<&str> {
        // TODO
        None        
    }

    fn can_resize(&self) -> bool {
        false
    }

    fn resize_view(&self, _width: f64, _height: f64) -> bool {
        false
    }

    fn start_parameter_change(&self, id: ParameterId) {
        self.sending_parameter_change_from_editor.store(true, Ordering::Release);
        unsafe { (self.start_parameter_change)(self.context, id); }
        self.sending_parameter_change_from_editor.store(false, Ordering::Release);
    }

    fn change_parameter_value(&self, id: ParameterId, normalized: ParameterValue) {
        self.sending_parameter_change_from_editor.store(true, Ordering::Release);
        unsafe { (self.change_parameter_value)(self.context, id, normalized as _); }
        self.sending_parameter_change_from_editor.store(false, Ordering::Release);

        // Update cached value
        let index = self.parameter_index_from_id.get(&id).unwrap();
        let mut cached_parameters = self.cached_parameters.lock().unwrap();
        let parameter = cached_parameters.get_mut(*index).unwrap();

        parameter.value = (normalized * parameter_multiplier(&parameter.info)) as _;
    }

    fn end_parameter_change(&self, id: ParameterId) {
        self.sending_parameter_change_from_editor.store(true, Ordering::Release);
        unsafe { (self.end_parameter_change)(self.context, id); }
        self.sending_parameter_change_from_editor.store(false, Ordering::Release);
    }
    
    fn mark_state_dirty(&self) {
        // TODO
    }
}

unsafe impl Send for Auv3Host {}
