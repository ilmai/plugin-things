use std::cell::RefCell;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::thread::ThreadId;
use std::{sync::Arc, any::Any, rc::Rc};
use std::sync::{Mutex, RwLock, Weak, mpsc};

use i_slint_core::window::WindowAdapter;
use nih_plug::prelude::*;
use plugin_canvas::event::EventResponse;
use plugin_canvas::{window::WindowAttributes, Event};
use raw_window_handle_0_4::HasRawWindowHandle;
use slint_interpreter::{ComponentHandle, ComponentInstance};

use crate::window_adapter::{Context, ParameterChangeSender, ParameterChange};
use crate::{platform::PluginCanvasPlatform, window_adapter::{WINDOW_TO_SLINT, WINDOW_ADAPTER_FROM_SLINT, PluginCanvasWindowAdapter}, raw_window_handle_adapter::RawWindowHandleAdapter};

pub struct SlintEditor<B, E>
where
    B: Fn() -> ComponentInstance,
    E: Fn(&ComponentInstance, &Event) -> EventResponse,
{
    window_attributes: WindowAttributes,
    os_scale_factor: RwLock<f32>,
    parameter_globals_name: String,
    component_builder: B,
    event_handler: E,
    editor_handle: Mutex<Option<Weak<EditorHandle>>>,
    param_map: Vec<(String, ParamPtr, String)>,
    parameter_change_sender: RefCell<Option<ParameterChangeSender>>,
}

impl<B, E> SlintEditor<B, E>
where
    B: Fn() -> ComponentInstance,
    E: Fn(&ComponentInstance, &Event) -> EventResponse,
{
    pub fn new(
        window_attributes: WindowAttributes,
        params: &impl Params,
        parameter_globals_name: impl AsRef<str>,
        component_builder: B,
        event_handler: E,
    ) -> Self {
        Self {
            window_attributes,
            os_scale_factor: RwLock::new(1.0),
            parameter_globals_name: parameter_globals_name.as_ref().into(),
            component_builder,
            event_handler,
            editor_handle: Default::default(),
            param_map: params.param_map(),
            parameter_change_sender: Default::default(),
        }
    }
}

impl<B, E> Editor for SlintEditor<B, E>
where
    B: Fn() -> ComponentInstance + Clone + Send + 'static,
    E: Fn(&ComponentInstance, &Event) -> EventResponse + Clone + Send + 'static,
{
    fn spawn(&self, parent: ParentWindowHandle, context: Arc<dyn GuiContext>) -> Box<dyn Any + Send> {
        let editor_handle = Arc::new(EditorHandle::new());
        let raw_window_handle_adapter = RawWindowHandleAdapter::from(parent.raw_window_handle());
        let window_attributes = self.window_attributes.clone();

        let (parameter_change_sender, parameter_change_receiver) = mpsc::channel();
        *self.parameter_change_sender.borrow_mut() = Some(parameter_change_sender);

        plugin_canvas::Window::open(
            raw_window_handle_adapter,
            window_attributes,
            *self.os_scale_factor.read().unwrap() as f64,
            {
                let editor_handle = Arc::downgrade(&editor_handle.clone());
                let event_handler = self.event_handler.clone();

                Box::new(move |event| {
                    if let Some(editor_handle) = editor_handle.upgrade() {
                        match editor_handle.on_event(&event) {
                            EventResponse::Ignored => {
                                editor_handle.window_adapter().with_context(|context| {
                                    event_handler(&context.component, &event)
                                })
                            },

                            response => response,
                        }
                    } else {
                        EventResponse::Ignored
                    }
                })
            },
            {
                let editor_handle = editor_handle.clone();
                let component_builder = self.component_builder.clone();
                let param_map = self.param_map.clone();
                let parameter_globals_name = self.parameter_globals_name.clone();
                let gui_context = context.clone();

                Box::new(move |window| {
                    // It's ok if this fails as it just means it has already been set
                    slint::platform::set_platform(Box::new(PluginCanvasPlatform)).ok();

                    WINDOW_TO_SLINT.set(Some(Box::new(window)));

                    let component = component_builder();
                    let component_definition = component.definition();
                    component.window().show().unwrap();
            
                    let param_map = param_map.iter()
                        .map(|(name, param_ptr, _)| {
                            (name.clone(), *param_ptr)
                        })
                        .collect();

                    let context = Context {
                        component,
                        component_definition,
                        param_map: Rc::new(param_map),
                        parameter_globals_name,
                        gui_context,
                        parameter_change_receiver,
                    };

                    let window_adapter = WINDOW_ADAPTER_FROM_SLINT.take().unwrap();
                    window_adapter.set_context(context);

                    editor_handle.set_window_adapter(window_adapter);
                })
            }
        ).unwrap();

        let weak_editor_handle = Arc::downgrade(&editor_handle);
        *self.editor_handle.lock().unwrap() = Some(weak_editor_handle);
        Box::new(editor_handle)
    }

    fn size(&self) -> (u32, u32) {
        let size = self.window_attributes.size * self.window_attributes.scale;
        (size.width as u32, size.height as u32)
    }

    fn set_scale_factor(&self, factor: f32) -> bool {
        *self.os_scale_factor.write().unwrap() = factor;
        true
    }

    fn param_value_changed(&self, id: &str, _normalized_value: f32) {
        let parameter_change_sender = self.parameter_change_sender.borrow();
        let id = id.to_string();

        parameter_change_sender.as_ref().unwrap().send(ParameterChange::ValueChanged { id }).unwrap();
    }

    fn param_modulation_changed(&self, id: &str, _modulation_offset: f32) {
        let parameter_change_sender = self.parameter_change_sender.borrow();
        let id = id.to_string();

        parameter_change_sender.as_ref().unwrap().send(ParameterChange::ModulationChanged { id }).unwrap();
    }

    fn param_values_changed(&self) {
        let parameter_change_sender = self.parameter_change_sender.borrow();
        parameter_change_sender.as_ref().unwrap().send(ParameterChange::AllValuesChanged).unwrap();
    }
}

struct EditorHandle {
    window_adapter_thread: Mutex<Option<ThreadId>>,
    window_adapter_ptr: AtomicPtr<PluginCanvasWindowAdapter>,
}

impl EditorHandle {
    fn new() -> Self {
        Self {
            window_adapter_thread: Default::default(),
            window_adapter_ptr: Default::default(),
        }
    }

    fn window_adapter(&self) -> &PluginCanvasWindowAdapter {
        assert!(*self.window_adapter_thread.lock().unwrap() == Some(std::thread::current().id()));

        let window_adapter_ptr = self.window_adapter_ptr.load(Ordering::Relaxed);
        assert!(!window_adapter_ptr.is_null());
        unsafe { &*window_adapter_ptr }
    }

    fn set_window_adapter(&self, window_adapter: Rc<PluginCanvasWindowAdapter>) {
        // Store thread id as we should never call anything in window adapter from other threads
        *self.window_adapter_thread.lock().unwrap() = Some(std::thread::current().id());
        self.window_adapter_ptr.store(Rc::into_raw(window_adapter) as _, Ordering::Relaxed);
    }

    fn on_event(&self, event: &Event) -> EventResponse {
        self.window_adapter().on_event(event)
    } 
}

impl Drop for EditorHandle {
    fn drop(&mut self) {
        let window_adapter_ptr = self.window_adapter_ptr.load(Ordering::Relaxed);
        let rc = unsafe { Rc::from_raw(window_adapter_ptr) };
        rc.window().dispatch_event(i_slint_core::platform::WindowEvent::CloseRequested);
    }
}
