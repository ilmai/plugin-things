use std::{ffi::c_void, sync::{atomic::{AtomicBool, Ordering}, Arc}};

use crate::{Host, ParameterId, ParameterValue};

pub struct Auv3Host {
    context: *mut c_void,
    start_parameter_change: unsafe extern "C-unwind" fn(*mut c_void, u32),
    change_parameter_value: unsafe extern "C-unwind" fn(*mut c_void, u32, f32),
    end_parameter_change: unsafe extern "C-unwind" fn(*mut c_void, u32),
    sending_parameter_change_from_editor: Arc<AtomicBool>,
}

impl Auv3Host {
    pub fn new(
        editor_context: *mut c_void,
        start_parameter_change: unsafe extern "C-unwind" fn(*mut c_void, u32),
        change_parameter_value: unsafe extern "C-unwind" fn(*mut c_void, u32, f32),
        end_parameter_change: unsafe extern "C-unwind" fn(*mut c_void, u32),
        sending_parameter_change_from_editor: Arc<AtomicBool>,
    ) -> Self
    {
        Self {
            context: editor_context,
            start_parameter_change,
            end_parameter_change,
            change_parameter_value,
            sending_parameter_change_from_editor,
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
