use std::{cell::RefCell, rc::Rc};

use vst3::{ComPtr, Steinberg::{kResultOk, IPlugFrameTrait, IPlugView, ViewRect, Vst::{IComponentHandler, IComponentHandler2, IComponentHandler2Trait, IComponentHandlerTrait, RestartFlags_::kParamValuesChanged}}};

use crate::{host::Host, parameters::ParameterValue, ParameterId, Parameters, Plugin};

use super::view::ViewContext;

pub struct Vst3Host<P: Plugin> {
    plugin: Rc<RefCell<Option<P>>>,
    component_handler: Rc<RefCell<Option<ComPtr<IComponentHandler>>>>,
    plug_view: ComPtr<IPlugView>,
    view_context: Rc<RefCell<ViewContext>>,
}

impl<P: Plugin> Vst3Host<P> {
    pub fn new(
        plugin: Rc<RefCell<Option<P>>>,
        handler: Rc<RefCell<Option<ComPtr<IComponentHandler>>>>,
        plug_view: ComPtr<IPlugView>,
        view_context: Rc<RefCell<ViewContext>>,
    ) -> Self {
        Self {
            plugin,
            component_handler: handler,
            plug_view,
            view_context,
        }
    }
}

impl<P: Plugin> Host for Vst3Host<P> {
    fn can_resize(&self) -> bool {
        true
    }

    fn resize_view(&self, width: f64, height: f64) -> bool {
        let view_context = self.view_context.borrow();
        let Some(frame) = view_context.frame.as_ref() else {
            return false;
        };

        let mut size = ViewRect {
            left: 0,
            top: 0,
            right: width as _,
            bottom: height as _,
        };

        let result = unsafe { frame.resizeView(self.plug_view.as_ptr(), &mut size) };
        result == kResultOk
    }

    fn start_parameter_change(&self, id: ParameterId) {
        if let Some(handler) = self.component_handler.borrow_mut().as_mut() {
            unsafe { handler.beginEdit(id) };
        }
    }

    fn change_parameter_value(&self, id: ParameterId, normalized: ParameterValue) {
        {
            let plugin = self.plugin.borrow();
            let Some(plugin) = plugin.as_ref() else {
                return;
            };

            plugin.with_parameters(|parameters| {
                let parameter = parameters.get(id).unwrap();
                parameter.set_normalized_value(normalized).unwrap();
            });
            
            // release plugin.borrow before calling performEdit
        } 

        if let Some(handler) = self.component_handler.borrow_mut().as_mut() {
            unsafe { handler.performEdit(id, normalized) };
        }
    }

    fn end_parameter_change(&self, id: ParameterId) {
        if let Some(handler) = self.component_handler.borrow_mut().as_mut() {
            unsafe { handler.endEdit(id) };
        }
    }
    
    fn reload_parameters(&self) {
        if let Some(handler) = self.component_handler.borrow_mut().as_mut() {
            unsafe { handler.restartComponent(kParamValuesChanged as _) };
        }        
    }

    fn mark_state_dirty(&self) {        
        if let Some(handler) = self.component_handler.borrow_mut().as_mut() && let Some(handler2) = handler.cast::<IComponentHandler2>() {
            unsafe { handler2.setDirty(1) };
        }
    }
}
