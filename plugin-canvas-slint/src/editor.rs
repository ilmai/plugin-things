use std::{ptr::null_mut, rc::Rc, sync::{atomic::{AtomicPtr, Ordering}, Mutex}, thread::ThreadId};

use raw_window_handle::RawWindowHandle;
use plugin_canvas::{event::EventResponse, window::WindowAttributes, Event};

use crate::{platform::PluginCanvasPlatform, plugin_component_handle::PluginComponentHandle, window_adapter::{Context, PluginCanvasWindowAdapter, WINDOW_ADAPTER_FROM_SLINT, WINDOW_TO_SLINT}};

pub struct SlintEditor {
}

impl SlintEditor {
    pub fn open<C, B>(
        parent: RawWindowHandle,
        window_attributes: WindowAttributes,
        component_builder: B
    ) -> Rc<EditorHandle>
    where
        C: PluginComponentHandle + 'static,
        B: Fn(Rc<plugin_canvas::Window>) -> C + 'static,
    {
        let editor_handle = Rc::new(EditorHandle::new());

        let window = plugin_canvas::Window::open(
            parent,
            window_attributes.clone(),
            {
                let editor_handle = Rc::downgrade(&editor_handle.clone());

                Box::new(move |event| {
                    let Some(editor_handle) = editor_handle.upgrade() else {
                        return EventResponse::Ignored;
                    };

                    editor_handle.on_event(&event);
                        
                    let Some(window_adapter) = editor_handle.window_adapter() else {
                        return EventResponse::Ignored;
                    };

                    window_adapter.with_context(|context| {
                        context.component.on_event(&event)
                    })
                })
            },
        ).unwrap();

        let editor_handle = editor_handle.clone();

        // It's ok if this fails as it just means it has already been set
        slint::platform::set_platform(Box::new(PluginCanvasPlatform)).ok();

        let window = Rc::new(window);
        WINDOW_TO_SLINT.set(Some(window.clone()));

        let component = component_builder(window);
        component.window().show().unwrap();

        let context = Context {
            component: Box::new(component),
        };

        let window_adapter = WINDOW_ADAPTER_FROM_SLINT.take().unwrap();
        window_adapter.set_context(context);

        editor_handle.set_window_adapter(window_adapter);
        editor_handle
    }
}

pub struct EditorHandle {
    window_adapter_thread: Mutex<Option<ThreadId>>,
    window_adapter_ptr: AtomicPtr<PluginCanvasWindowAdapter>,
}

impl EditorHandle {
    pub fn on_frame(&self) {
        self.on_event(&Event::Draw);
    }

    fn new() -> Self {
        Self {
            window_adapter_thread: Default::default(),
            window_adapter_ptr: Default::default(),
        }
    }

    fn window_adapter(&self) -> Option<&PluginCanvasWindowAdapter> {
        // Don't allow from invalid threads
        if *self.window_adapter_thread.lock().unwrap() != Some(std::thread::current().id()) {
            return None;
        }

        let window_adapter_ptr = self.window_adapter_ptr.load(Ordering::Relaxed);
        if window_adapter_ptr.is_null() {
            return None;
        }

        unsafe { Some(&*window_adapter_ptr) }
    }

    fn set_window_adapter(&self, window_adapter: Rc<PluginCanvasWindowAdapter>) {
        // Store thread id as we should never call anything in window adapter from other threads
        *self.window_adapter_thread.lock().unwrap() = Some(std::thread::current().id());
        self.window_adapter_ptr.store(Rc::into_raw(window_adapter) as _, Ordering::Relaxed);
    }

    fn on_event(&self, event: &Event) -> EventResponse {
        if let Some(window_adapter) = self.window_adapter() {
            window_adapter.on_event(event)
        } else {
            EventResponse::Ignored
        }
    } 
}

impl Drop for EditorHandle {
    fn drop(&mut self) {
        let window_adapter_ptr = self.window_adapter_ptr.swap(null_mut(), Ordering::Relaxed);
        unsafe { Rc::from_raw(window_adapter_ptr) };
    }
}
