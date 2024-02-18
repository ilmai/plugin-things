use std::cell::RefCell;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::thread::ThreadId;
use std::{sync::Arc, any::Any, rc::Rc};
use std::sync::{Mutex, RwLock, Weak, mpsc};

use nih_plug::prelude::*;
use plugin_canvas::event::EventResponse;
use plugin_canvas::{window::WindowAttributes, Event};
use raw_window_handle::HasRawWindowHandle;

use crate::plugin_component_handle::PluginComponentHandleParameterEvents;
use crate::window_adapter::{Context, ParameterChangeSender, ParameterChange};
use crate::{platform::PluginCanvasPlatform, window_adapter::{WINDOW_TO_SLINT, WINDOW_ADAPTER_FROM_SLINT, PluginCanvasWindowAdapter}};

pub struct SlintEditor<C, B>
where
    C: PluginComponentHandleParameterEvents,
    B: Fn(Arc<plugin_canvas::Window>, Arc<dyn GuiContext>) -> C,
{
    window_attributes: WindowAttributes,
    os_scale: RwLock<f32>,
    component_builder: B,

    editor_handle: Mutex<Option<Weak<EditorHandle>>>,
    parameter_change_sender: RefCell<Option<ParameterChangeSender>>,
}

impl<C, B> SlintEditor<C, B>
where
    C: PluginComponentHandleParameterEvents,
    B: Fn(Arc<plugin_canvas::Window>, Arc<dyn GuiContext>) -> C,
{
    pub fn new(
        window_attributes: WindowAttributes,
        component_builder: B,
    ) -> Self {
        Self {
            window_attributes,
            os_scale: RwLock::new(1.0),
            component_builder,

            editor_handle: Default::default(),
            parameter_change_sender: Default::default(),
        }
    }
}

impl<C, B> Editor for SlintEditor<C, B>
where
    C: PluginComponentHandleParameterEvents + 'static,
    B: Fn(Arc<plugin_canvas::Window>, Arc<dyn GuiContext>) -> C + Clone + Send + 'static,
{
    fn spawn(&self, parent: ParentWindowHandle, gui_context: Arc<dyn GuiContext>) -> Box<dyn Any + Send> {
        let editor_handle = Arc::new(EditorHandle::new());
        let window_attributes = self.window_attributes.clone();

        let (parameter_change_sender, parameter_change_receiver) = mpsc::channel();
        *self.parameter_change_sender.borrow_mut() = Some(parameter_change_sender);

        plugin_canvas::Window::open(
            parent.raw_window_handle(),
            window_attributes,
            *self.os_scale.read().unwrap() as f64,
            {
                let editor_handle = Arc::downgrade(&editor_handle.clone());

                Box::new(move |event| {
                    if let Some(editor_handle) = editor_handle.upgrade() {
                        editor_handle.on_event(&event);
                        
                        editor_handle.window_adapter().with_context(|context| {
                            context.component.on_event(&event)
                        })
                    } else {
                        EventResponse::Ignored
                    }
                })
            },
            {
                let editor_handle = editor_handle.clone();
                let component_builder = self.component_builder.clone();
                let gui_context = gui_context.clone();

                Box::new(move |window| {
                    // It's ok if this fails as it just means it has already been set
                    slint::platform::set_platform(Box::new(PluginCanvasPlatform)).ok();

                    let window = Arc::new(window);
                    WINDOW_TO_SLINT.set(Some(window.clone()));

                    let component = component_builder(window, gui_context.clone());

                    component.on_start_parameter_change({
                        let gui_context = gui_context.clone();
                        let param_map = component.param_map().clone();
            
                        move |parameter_id| {
                            let param_ptr = param_map.get(&parameter_id).expect(&format!("Couldn't find parameter {parameter_id}"));
                            unsafe { gui_context.raw_begin_set_parameter(*param_ptr) };
                        }
                    });
            
                    component.on_parameter_changed({
                        let gui_context = gui_context.clone();
                        let param_map = component.param_map().clone();
            
                        move |parameter_id, value| {
                            let param_ptr = param_map.get(&parameter_id).expect(&format!("Couldn't find parameter {parameter_id}"));
                            unsafe { gui_context.raw_set_parameter_normalized(*param_ptr, value) };
                        }
                    });
            
                    component.on_end_parameter_change({
                        let gui_context = gui_context.clone();
                        let param_map = component.param_map().clone();
            
                        move |parameter_id| {
                            let param_ptr = param_map.get(&parameter_id).expect(&format!("Couldn't find parameter {parameter_id}"));
                            unsafe { gui_context.raw_end_set_parameter(*param_ptr) };
                        }
                    });
            
                    component.on_set_parameter_string({
                        let gui_context = gui_context.clone();
                        let param_map = component.param_map().clone();
            
                        move |parameter_id, string| {
                            let param_ptr = param_map.get(&parameter_id).expect(&format!("Couldn't find parameter {parameter_id}"));

                            unsafe {
                                if let Some(value) = param_ptr.string_to_normalized_value(&string) {
                                    gui_context.raw_begin_set_parameter(*param_ptr);
                                    gui_context.raw_set_parameter_normalized(*param_ptr, value);
                                    gui_context.raw_end_set_parameter(*param_ptr);
                                }    
                            }
                        }
                    });
            
                    component.window().show().unwrap();

                    let context = Context {
                        gui_context,
                        parameter_change_receiver,
                        component: Box::new(component),
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
        let size = self.window_attributes.scaled_size();
        (size.width as u32, size.height as u32)
    }

    fn set_scale_factor(&self, factor: f32) -> bool {
        *self.os_scale.write().unwrap() = factor;
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
        self.on_event(&Event::Close);

        let window_adapter_ptr = self.window_adapter_ptr.swap(null_mut(), Ordering::Relaxed);
        unsafe { Rc::from_raw(window_adapter_ptr) };
    }
}
