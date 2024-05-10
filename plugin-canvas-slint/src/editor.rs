use std::{ptr::null_mut, rc::Rc, sync::{atomic::{AtomicPtr, Ordering}, Arc, Mutex}, thread::ThreadId};

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
    ) -> Arc<EditorHandle>
    where
        C: PluginComponentHandle + 'static,
        B: Fn(Arc<plugin_canvas::Window>) -> C + Send + 'static,
    {
        let editor_handle = Arc::new(EditorHandle::new());

        plugin_canvas::Window::open(
            parent,
            window_attributes.clone(),
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

                Box::new(move |window| {
                    // It's ok if this fails as it just means it has already been set
                    slint::platform::set_platform(Box::new(PluginCanvasPlatform)).ok();

                    let window = Arc::new(window);
                    WINDOW_TO_SLINT.set(Some(window.clone()));

                    let component = component_builder(window);
                    component.window().show().unwrap();

                    let context = Context {
                        component: Box::new(component),
                    };

                    let window_adapter = WINDOW_ADAPTER_FROM_SLINT.take().unwrap();
                    window_adapter.set_context(context);

                    editor_handle.set_window_adapter(window_adapter);
                })
            }
        ).unwrap();

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
        let window_adapter_ptr = self.window_adapter_ptr.swap(null_mut(), Ordering::Relaxed);
        unsafe { Rc::from_raw(window_adapter_ptr) };
    }
}
