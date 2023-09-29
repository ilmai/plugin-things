use std::sync::atomic::{AtomicPtr, Ordering};
use std::thread::ThreadId;
use std::{sync::Arc, any::Any, rc::Rc};
use std::sync::{Mutex, RwLock, Weak};

use i_slint_core::window::WindowAdapter;
use nih_plug::prelude::*;
use plugin_canvas::{window::WindowAttributes, Event};
use raw_window_handle_0_4::HasRawWindowHandle;
use slint_interpreter::{ComponentHandle, ComponentInstance};

use crate::window_adapter::Context;
use crate::{platform::PluginCanvasPlatform, window_adapter::{WINDOW_TO_SLINT, WINDOW_ADAPTER_FROM_SLINT, PluginCanvasWindowAdapter}, raw_window_handle_adapter::RawWindowHandleAdapter};

pub struct SlintEditor<F>
where
    F: Fn() -> ComponentInstance,
{
    window_attributes: WindowAttributes,
    os_scale_factor: RwLock<f32>,
    component_builder: F,
    editor_handle: Mutex<Option<Weak<EditorHandle>>>,
    param_map: Vec<(String, ParamPtr, String)>,
}

impl<F> SlintEditor<F>
where
    F: Fn() -> ComponentInstance,
{
    pub fn new(window_attributes: WindowAttributes, params: &impl Params, component_builder: F) -> Self {
        Self {
            window_attributes,
            os_scale_factor: RwLock::new(1.0),
            component_builder,
            editor_handle: Default::default(),
            param_map: params.param_map(),
        }
    }
}

impl<F> Editor for SlintEditor<F>
where
    F: Fn() -> ComponentInstance + Clone + Send + 'static,
{
    fn spawn(&self, parent: ParentWindowHandle, context: Arc<dyn GuiContext>) -> Box<dyn Any + Send> {
        let editor_handle = Arc::new(EditorHandle::new(context.clone()));
        let raw_window_handle_adapter = RawWindowHandleAdapter::from(parent.raw_window_handle());
        let mut window_attributes = self.window_attributes.clone();
        window_attributes.scale *= *self.os_scale_factor.read().unwrap() as f64;

        plugin_canvas::Window::open(
            raw_window_handle_adapter,
            window_attributes,
            {
                let editor_handle = Arc::downgrade(&editor_handle.clone());
                Box::new(move |event| {
                    if let Some(editor_handle) = editor_handle.upgrade() {
                        editor_handle.on_event(event)
                    }
                })
            },
            {
                let editor_handle = editor_handle.clone();
                let component_builder = self.component_builder.clone();
                let param_map = self.param_map.clone();
                let gui_context = context.clone();

                Box::new(move |window| {
                    // It's ok if this fails as it just means it has already been set
                    slint::platform::set_platform(Box::new(PluginCanvasPlatform)).ok();

                    WINDOW_TO_SLINT.with(move |next_window| { *next_window.borrow_mut() = Some(Box::new(window)); });

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
                        gui_context,
                    };

                    let window_adapter = WINDOW_ADAPTER_FROM_SLINT.with(|window_adapter| window_adapter.take().unwrap());
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

    fn param_value_changed(&self, _id: &str, _normalized_value: f32) {
    }

    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {
    }

    fn param_values_changed(&self) {
    }
}

struct EditorHandle {
    window_adapter_thread: Mutex<Option<ThreadId>>,
    window_adapter_ptr: AtomicPtr<PluginCanvasWindowAdapter>,
    _gui_context: Arc<dyn GuiContext>,
}

impl EditorHandle {
    pub fn new(gui_context: Arc<dyn GuiContext>) -> Self {
        Self {
            window_adapter_thread: Default::default(),
            window_adapter_ptr: Default::default(),
            _gui_context: gui_context,
        }
    }

    fn set_window_adapter(&self, window_adapter: Rc<PluginCanvasWindowAdapter>) {
        // Store thread id as we should never call anything in window adapter from other threads
        *self.window_adapter_thread.lock().unwrap() = Some(std::thread::current().id());
        self.window_adapter_ptr.store(Rc::into_raw(window_adapter) as _, Ordering::Relaxed);
    }

    fn on_event(&self, event: Event) {
        let window_adapter_ptr = self.window_adapter_ptr.load(Ordering::Relaxed);
        assert!(*self.window_adapter_thread.lock().unwrap() == Some(std::thread::current().id()));
        assert!(!window_adapter_ptr.is_null());

        let window_adapter = unsafe { &*window_adapter_ptr };
        window_adapter.on_event(event);
    } 
}

impl Drop for EditorHandle {
    fn drop(&mut self) {
        let window_adapter_ptr = self.window_adapter_ptr.load(Ordering::Relaxed);
        let rc = unsafe { Rc::from_raw(window_adapter_ptr) };
        rc.window().dispatch_event(i_slint_core::platform::WindowEvent::CloseRequested);
    }
}
